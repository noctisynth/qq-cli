"""Run the real installer only in private temporary directories, without QQ or Keychain."""
import hashlib
import json
import os
import platform
import subprocess
import sys
import unittest
from pathlib import Path

import test_macos_release as fixtures


@unittest.skipUnless(sys.platform == "darwin" and platform.machine() == "arm64",
                     "installer requires native Apple Silicon")
class InstallTests(unittest.TestCase):
    package = fixtures.DistributionTests.package
    checksums = fixtures.DistributionTests.checksums
    def setUp(self):
        fixtures.DistributionTests.setUp(self)
        self.installer = Path(__file__).resolve().parents[1] / "install-macos.sh"
        self.directory = self.root / "installed"
        self.bin = self.root / "bin"
        self.archive = self.package("release")
        self.command = self.bin / "qq-cli"
        self.environment = dict(os.environ, TMPDIR=str(self.root))

    def invoke(self, *args, success=True, archive=None, remote=False, inline=False):
        entry = (["/bin/bash", "-c", self.installer.read_text(), "--"] if inline
                 else [str(self.installer)])
        command = entry + ["--directory", str(self.directory),
                           "--bin-directory", str(self.bin)]
        if "--uninstall" not in args and not remote:
            archive = archive or self.archive
            command += ["--archive", str(archive), "--checksums", str(archive.parent / "SHA256SUMS")]
        result = subprocess.run(command + list(args), env=self.environment,
                                text=True, capture_output=True)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        return result

    def test_install_repeat_upgrade_and_uninstall_keep_original_binaries_and_data(self):
        self.invoke()
        original = self.command.resolve()
        self.assertEqual(original.read_bytes(), self.executable.read_bytes())
        self.assertTrue(os.access(self.command, os.X_OK))
        self.invoke()
        self.assertEqual(self.command.resolve(), original)
        self.executable.write_bytes(self.executable.read_bytes() + b"upgrade")
        new = self.package("upgrade", "1.2.4")
        output = self.invoke(archive=new).stdout
        self.assertIn("Previous CLI retained for recovery", output)
        self.assertNotEqual(self.command.resolve(), original)
        self.assertTrue(original.is_file())
        marker = self.directory / "versions" / "keep-user-data"
        marker.write_text("preserve")
        self.invoke("--uninstall", inline=True)
        self.invoke("--uninstall")
        self.assertFalse(self.command.is_symlink())
        self.assertTrue(original.is_file())
        self.assertEqual(marker.read_text(), "preserve")

    def test_legacy_layout_upgrades_to_complete_task_guides(self):
        import zipfile
        tasks = {"skills/qq-cli/references/" + name + ".md"
                 for name in ("account", "assets", "contact", "files", "group")}
        legacy = self.root / "legacy" / self.archive.name
        legacy.parent.mkdir()
        with zipfile.ZipFile(self.archive) as source, zipfile.ZipFile(legacy, "w") as target:
            for entry in source.infolist():
                if entry.filename not in tasks:
                    target.writestr(entry, source.read(entry))
        self.checksums(legacy)
        self.invoke(archive=legacy)
        original = self.command.resolve()
        self.invoke()
        current = self.command.resolve()
        self.assertNotEqual(current, original)
        self.assertTrue(original.is_file())
        for name in tasks:
            self.assertTrue((current.parent / name).is_file())

    def test_incomplete_task_guides_are_rejected_before_installation(self):
        import zipfile
        partial = self.root / "partial" / self.archive.name
        partial.parent.mkdir()
        with zipfile.ZipFile(self.archive) as source, zipfile.ZipFile(partial, "w") as target:
            for entry in source.infolist():
                if entry.filename != "skills/qq-cli/references/files.md":
                    target.writestr(entry, source.read(entry))
        self.checksums(partial)
        self.invoke(archive=partial, success=False)
        self.assertFalse(self.directory.exists())

    def test_hash_and_foreign_command_failures_preserve_existing_installation(self):
        self.invoke()
        original = os.readlink(self.command)
        self.archive.write_bytes(self.archive.read_bytes() + b"changed")
        self.invoke(success=False)
        self.assertEqual(os.readlink(self.command), original)
        self.command.unlink()
        self.command.write_text("foreign command")
        self.invoke("--uninstall", success=False)
        self.assertEqual(self.command.read_text(), "foreign command")

    def test_modified_slot_links_and_concurrent_installer_are_not_replaced(self):
        self.invoke()
        slot = self.command.resolve().parent
        original = self.command.resolve()
        original.write_bytes(b"modified")
        self.invoke(success=False)
        self.assertEqual(original.read_bytes(), b"modified")
        original.unlink()
        original.symlink_to(self.executable)
        self.invoke(success=False)
        self.assertTrue(original.is_symlink())
        lock = self.directory / ".install-lock"
        lock.mkdir()
        self.invoke("--uninstall", success=False)
        self.assertTrue(lock.is_dir())
        self.assertTrue(slot.is_dir())

    def test_archive_paths_and_unowned_directories_are_rejected_before_install(self):
        import zipfile
        with zipfile.ZipFile(self.archive, "a") as package:
            package.writestr("../outside", b"unexpected")
        self.checksums(self.archive)
        self.invoke(success=False)
        self.assertFalse(self.directory.exists())
        self.archive = self.package("clean")
        self.directory.mkdir()
        unrelated = self.directory / "unrelated"
        unrelated.write_text("preserve")
        self.invoke(success=False)
        self.assertEqual(unrelated.read_text(), "preserve")

    def test_quarantine_is_retained_without_executing_the_installed_program(self):
        quarantine = "0081;00000000;qq-cli-synthetic;"
        subprocess.run(["/usr/bin/xattr", "-w", "com.apple.quarantine", quarantine,
                        str(self.archive)], check=True)
        self.invoke()
        installed = self.command.resolve()
        actual = subprocess.run(["/usr/bin/xattr", "-p", "com.apple.quarantine", str(installed)],
                                capture_output=True, text=True, check=True).stdout.strip()
        self.assertEqual(actual.split(";")[0], quarantine.split(";")[0])
        self.assertEqual(len(actual.split(";")), 4)

    def test_release_metadata_uses_exact_platform_assets_and_rejects_missing_or_foreign(self):
        mock = self.root / "mock"
        mock.mkdir()
        metadata = self.root / "release.json"
        origin = "https://github.com/noctisynth/qq-cli/releases/download/v1.2.3/"
        data = {"draft": False, "prerelease": False, "assets": [
            {"name": self.archive.name, "browser_download_url": origin + self.archive.name},
            {"name": "qq-cli-v1.2.3-windows-x86_64.zip",
             "browser_download_url": origin + "qq-cli-v1.2.3-windows-x86_64.zip"},
            {"name": "SHA256SUMS", "browser_download_url": origin + "SHA256SUMS"}]}
        sums = self.archive.parent / "SHA256SUMS"
        windows_digest = hashlib.sha256(b"synthetic Windows package").hexdigest()
        sums.write_text(sums.read_text() + windows_digest + "  qq-cli-v1.2.3-windows-x86_64.zip\n")
        curl = mock / "curl"
        curl.write_text("""#!/bin/bash
set -eu
url=''
output=''
while [[ $# -gt 0 ]]; do
  case "$1" in
    -o) output=$2; shift 2;;
    https://*) url=$1; shift;;
    *) shift;;
  esac
done
if [[ $url == */releases/latest ]]; then cat "$MOCK_METADATA"
elif [[ $url == */SHA256SUMS ]]; then cp "$MOCK_SUMS" "$output"
else cp "$MOCK_ARCHIVE" "$output"; fi
""")
        curl.chmod(0o755)
        self.environment.update(PATH=str(mock) + ":" + os.environ["PATH"],
                                MOCK_METADATA=str(metadata), MOCK_ARCHIVE=str(self.archive),
                                MOCK_SUMS=str(self.archive.parent / "SHA256SUMS"))
        for bad in ("missing", "foreign", "duplicate", "prerelease"):
            candidate = json.loads(json.dumps(data))
            if bad == "missing":
                candidate["assets"] = []
            elif bad == "foreign":
                candidate["assets"][0]["browser_download_url"] = "https://example.com/cli.zip"
            elif bad == "duplicate":
                candidate["assets"].append(candidate["assets"][0])
            else:
                candidate["prerelease"] = True
            metadata.write_text(json.dumps(candidate))
            self.invoke(remote=True, success=False)
            self.assertFalse(self.directory.exists())
        metadata.write_text(json.dumps(data))
        self.invoke(remote=True, inline=True)
        self.assertTrue(self.command.is_symlink())


if __name__ == "__main__":
    unittest.main()
