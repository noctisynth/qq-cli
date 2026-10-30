"""Synthetic distribution tests: no user state, credentials or upstream assets."""
import hashlib
import stat
import struct
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import macos_release as release


class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.executable = self.root / "synthetic-cli"
        self.executable.write_bytes(struct.pack("<10I", 0xFEEDFACF, 0x0100000C,
                                               0, 2, 1, 8, 0, 0, 1, 8))
        for name in ("README.md", "LICENSE-MIT", "LICENSE-APACHE"):
            (self.root / name).write_text(name)
        for name in release.SKILL_FILES:
            path = self.root / "skills/qq-cli" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(name)
        # Neither tree may enter an archive, even if present beside the inputs.
        for name in ("work/private.txt", ".agents/internal.md"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("not a distribution input")

    def package(self, output="output", version="1.2.3"):
        return release.package(self.root, self.executable, version, self.root / output)

    def checksums(self, archive):
        path = archive.parent / "SHA256SUMS"
        path.write_text(hashlib.sha256(archive.read_bytes()).hexdigest()
                        + "  " + archive.name + "\n")
        return path

    def test_reproducible_allowlisted_archive_with_executable_permissions(self):
        first, second = self.package("a"), self.package("b")
        self.assertEqual(first.read_bytes(), second.read_bytes())
        with zipfile.ZipFile(first) as archive:
            self.assertEqual(len(archive.infolist()), 4 + len(release.SKILL_FILES))
            self.assertEqual(archive.getinfo("qq-cli").external_attr >> 16,
                             stat.S_IFREG | 0o755)
            self.assertEqual(archive.read("qq-cli"), self.executable.read_bytes())
            self.assertFalse(any(name.startswith(("work/", ".agents/"))
                                 for name in archive.namelist()))

    def test_repository_skill_tree_and_installer_match_the_archive(self):
        repository = Path(__file__).resolve().parents[2]
        archive = release.package(repository, self.executable, "1.2.3", self.root / "repository")
        installer = (repository / "scripts/install-macos.sh").read_text()
        base = installer.split("<<'NAMES'\n", 1)[1].split("\nNAMES\n", 1)[0].splitlines()
        tasks = installer.split("<<'TASK_NAMES'\n", 1)[1].split("\nTASK_NAMES\n", 1)[0].splitlines()
        with zipfile.ZipFile(archive) as package:
            self.assertEqual(sorted(package.namelist()), sorted(base + tasks))

    def test_existing_outputs_and_invalid_versions_are_preserved(self):
        archive = self.package()
        original = archive.read_bytes()
        with self.assertRaises(ValueError):
            self.package()
        self.assertEqual(archive.read_bytes(), original)
        for version in ("../outside", "v1.2.3", "1.2.3\n"):
            with self.assertRaises(ValueError):
                self.package("invalid", version)
        self.assertFalse((self.root / "invalid").exists())

    def test_foreign_or_malformed_executables_never_create_output(self):
        original = self.executable.read_bytes()
        for data in (b"MZ", original[:4] + struct.pack("<I", 0x01000007) + original[8:],
                     original[:20] + struct.pack("<I", 1000) + original[24:],
                     original[:-4] + struct.pack("<I", 0)):
            self.executable.write_bytes(data)
            with self.assertRaises(ValueError):
                self.package()
            self.assertFalse((self.root / "output").exists())

    def test_unreviewed_skill_files_and_links_are_rejected(self):
        extra = self.root / "skills/qq-cli/private.txt"
        extra.write_text("unexpected")
        with self.assertRaises(ValueError):
            self.package()
        extra.unlink()
        extra.symlink_to(self.executable)
        with self.assertRaises(ValueError):
            self.package()

    def test_checksum_content_permissions_and_extra_entries_are_checked(self):
        expected = release.sources(self.root, self.executable)
        for change in ("checksum", "content", "permissions", "extra"):
            with self.subTest(change=change):
                archive = self.package(change)
                checksum = archive.parent / "SHA256SUMS"
                if change == "checksum":
                    checksum.write_text("incorrect\n")
                else:
                    with zipfile.ZipFile(archive) as original:
                        entries = [(entry, original.read(entry)) for entry in original.infolist()]
                    with zipfile.ZipFile(archive, "w") as target:
                        for entry, data in entries:
                            if entry.filename == "qq-cli":
                                if change == "content":
                                    data += b"changed"
                                elif change == "permissions":
                                    entry.external_attr = (stat.S_IFREG | 0o644) << 16
                            target.writestr(entry, data)
                        if change == "extra":
                            target.writestr("../outside", b"unexpected")
                    checksum = self.checksums(archive)
                with self.assertRaises(ValueError):
                    release.verify(archive, checksum, expected)


if __name__ == "__main__":
    unittest.main()
