"""Package only original CLI distribution files; never fetch or run QQ."""
import argparse
import hashlib
import re
import stat
import struct
import zipfile
from pathlib import Path


SKILL_FILES = (
    "SKILL.md", "references/shared.md", "references/discover.md",
    "references/read.md", "references/send.md", "references/media.md",
    "references/events.md", "references/runtime.md", "references/runtime-macos.md",
    "references/account.md", "references/contact.md", "references/group.md",
    "references/assets.md", "references/files.md",
)


def regular_bytes(path):
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError("Distribution inputs must be regular files")
    return path.read_bytes()


def executable_bytes(path):
    data = regular_bytes(path)
    if len(data) < 32:
        raise ValueError("Truncated Mach-O executable")
    magic, cpu, _, kind, count, size, _, _ = struct.unpack_from("<8I", data)
    if (magic, cpu, kind) != (0xFEEDFACF, 0x0100000C, 2):
        raise ValueError("Expected a thin arm64 Mach-O executable")
    if not count or size > len(data) - 32 or count > size // 8:
        raise ValueError("Invalid Mach-O load commands")
    end, offset = 32 + size, 32
    for _ in range(count):
        if offset + 8 > end:
            raise ValueError("Truncated Mach-O load command")
        _, length = struct.unpack_from("<2I", data, offset)
        if length < 8 or length % 8 or offset + length > end:
            raise ValueError("Invalid Mach-O load command length")
        offset += length
    if offset != end:
        raise ValueError("Unexpected Mach-O load command bytes")
    return data


def sources(root, executable):
    files = {"qq-cli": executable_bytes(executable)}
    for name in ("README.md", "LICENSE-MIT", "LICENSE-APACHE"):
        files[name] = regular_bytes(root / name)
    skill = root / "skills/qq-cli"
    if skill.is_symlink() or not skill.is_dir():
        raise ValueError("Skill root must be a regular directory")
    actual = set()
    for path in skill.rglob("*"):
        if path.is_symlink():
            raise ValueError("Skill links are not distribution inputs")
        if path.is_dir():
            continue
        actual.add(path.relative_to(skill).as_posix())
    if actual != set(SKILL_FILES):
        raise ValueError("Skill files differ from the distribution allowlist")
    for name in SKILL_FILES:
        files["skills/qq-cli/" + name] = regular_bytes(skill / name)
    return files


def verify(archive, checksums, expected):
    digest = hashlib.sha256(regular_bytes(archive)).hexdigest()
    if regular_bytes(checksums) != (digest + "  " + archive.name + "\n").encode("ascii"):
        raise ValueError("Distribution checksum mismatch")
    with zipfile.ZipFile(archive) as package:
        if sorted(package.namelist()) != sorted(expected):
            raise ValueError("Unexpected or duplicate distribution entries")
        for entry in package.infolist():
            mode = entry.external_attr >> 16
            required = stat.S_IFREG | (0o755 if entry.filename == "qq-cli" else 0o644)
            if entry.create_system != 3 or mode != required:
                raise ValueError("Incorrect distribution permissions")
            if package.read(entry) != expected[entry.filename]:
                raise ValueError("Distribution content mismatch")


def package(root, executable, version, output):
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version):
        raise ValueError("Invalid Cargo package version")
    files = sources(root, executable)
    output.mkdir(parents=True, exist_ok=True)
    if output.is_symlink() or not output.is_dir():
        raise ValueError("Output must be a regular directory")
    archive = output / ("qq-cli-v" + version + "-macos-aarch64.zip")
    checksums = output / "SHA256SUMS"
    if archive.exists() or checksums.exists():
        raise ValueError("Release output already exists; choose a new directory")
    # Exclusive creation preserves existing output, including concurrent builds.
    # An interrupted output is retained; it is never promoted or overwritten.
    with archive.open("xb") as stream, checksums.open("xb") as checksum_stream:
        with zipfile.ZipFile(stream, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as target:
            for name, data in sorted(files.items()):
                entry = zipfile.ZipInfo(name, (2000, 1, 1, 0, 0, 0))
                entry.create_system = 3
                entry.external_attr = (stat.S_IFREG | (0o755 if name == "qq-cli" else 0o644)) << 16
                entry.compress_type = zipfile.ZIP_DEFLATED
                target.writestr(entry, data)
        stream.flush()
        checksum_stream.write((hashlib.sha256(archive.read_bytes()).hexdigest()
                               + "  " + archive.name + "\n").encode("ascii"))
    verify(archive, checksums, files)
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        archive = package(Path(__file__).resolve().parent.parent,
                          args.executable, args.version, args.output)
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        parser.exit(1, str(error) + "\n")
    print("Checked macOS arm64 package: " + archive.name + " and SHA256SUMS")


if __name__ == "__main__":
    main()
