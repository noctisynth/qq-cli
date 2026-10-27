"""Cross-platform artifact handoff: checksums, versions and output preservation."""
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_assets


class ReleaseAssetsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.windows = self.root / "windows"
        self.macos = self.root / "macos"
        self.output = self.root / "release"
        self.inputs = {}
        for directory, platform in ((self.windows, "windows-x86_64"), (self.macos, "macos-aarch64")):
            directory.mkdir()
            name = "qq-cli-v1.2.3-" + platform + ".zip"
            data = ("synthetic checked archive " + platform).encode()
            (directory / name).write_bytes(data)
            digest = hashlib.sha256(data).hexdigest()
            (directory / "SHA256SUMS").write_bytes((digest + "  " + name + "\n").encode())
            self.inputs[name] = data

    def collect(self):
        return release_assets.collect("1.2.3", self.windows, self.macos, self.output)

    def test_packages_and_common_manifest_preserve_both_checked_archives(self):
        # Windows line endings are valid source manifests too.
        sums = self.windows / "SHA256SUMS"
        sums.write_bytes(sums.read_bytes().replace(b"\n", b"\r\n"))
        self.assertEqual(self.collect(), sorted(self.inputs))
        for name, data in self.inputs.items():
            self.assertEqual((self.output / name).read_bytes(), data)
        expected = "".join(hashlib.sha256(data).hexdigest() + "  " + name + "\n"
                           for name, data in sorted(self.inputs.items()))
        self.assertEqual((self.output / "SHA256SUMS").read_bytes(), expected.encode())

    def test_corrupt_archive_or_manifest_is_rejected_before_output(self):
        for directory in (self.windows, self.macos):
            sums = directory / "SHA256SUMS"
            original = sums.read_bytes()
            for bad in (b"incorrect\n", original + original):
                sums.write_bytes(bad)
                with self.assertRaises(ValueError):
                    self.collect()
                self.assertFalse(self.output.exists())
            sums.write_bytes(original)
        archive = next(self.macos.glob("*.zip"))
        archive.write_bytes(b"changed")
        with self.assertRaises(ValueError):
            self.collect()
        self.assertFalse(self.output.exists())

    def test_wrong_version_extra_files_and_links_are_rejected(self):
        with self.assertRaises(ValueError):
            release_assets.collect("1.2.4", self.windows, self.macos, self.output)
        extra = self.macos / "unreviewed.txt"
        extra.write_text("not a release asset")
        with self.assertRaises(ValueError):
            self.collect()
        extra.unlink()
        archive = next(self.macos.glob("*.zip"))
        original = self.root / "original.zip"
        archive.rename(original)
        archive.symlink_to(original)
        with self.assertRaises(ValueError):
            self.collect()
        self.assertFalse(self.output.exists())

    def test_existing_release_and_invalid_version_are_never_overwritten(self):
        self.collect()
        before = {p.name: p.read_bytes() for p in self.output.iterdir()}
        with self.assertRaises(ValueError):
            self.collect()
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.output.iterdir()})
        for version in ("../outside", "v1.2.3", "1.2.3\n"):
            with self.assertRaises(ValueError):
                release_assets.collect(version, self.windows, self.macos, self.root / "invalid")
        self.assertFalse((self.root / "invalid").exists())


if __name__ == "__main__":
    unittest.main()
