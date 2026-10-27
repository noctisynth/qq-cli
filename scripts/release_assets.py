"""Collect checked platform packages into one release asset set; never publish."""
import argparse
import hashlib
from pathlib import Path
import re
import stat


def regular_bytes(path):
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 256 * 1024 * 1024:
        raise ValueError("Release inputs must be bounded regular files")
    return path.read_bytes()


def platform_package(directory, name):
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError("Platform artifact directory must be a regular directory")
    if {path.name for path in directory.iterdir()} != {name, "SHA256SUMS"}:
        raise ValueError("Platform artifacts must contain the matching version and checksums only")
    data = regular_bytes(directory / name)
    digest = hashlib.sha256(data).hexdigest()
    lines = regular_bytes(directory / "SHA256SUMS").decode("ascii").splitlines()
    if lines != [digest + "  " + name]:
        raise ValueError("Platform package checksum mismatch")
    return data, digest


def collect(version, windows, macos, output):
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version):
        raise ValueError("Invalid Cargo package version")
    packages = {}
    for directory, platform in ((windows, "windows-x86_64"), (macos, "macos-aarch64")):
        name = "qq-cli-v" + version + "-" + platform + ".zip"
        packages[name] = platform_package(directory, name)
    if output.is_symlink():
        raise ValueError("Release output must not be a link")
    output.mkdir(parents=True, exist_ok=True)
    targets = [output / name for name in packages] + [output / "SHA256SUMS"]
    if any(path.exists() or path.is_symlink() for path in targets):
        raise ValueError("Release output already exists; choose a new directory")
    # Both source packages are verified before any release file is created.
    # Keep exclusive creation and retain partial output on failure.
    for name, (data, _) in packages.items():
        with (output / name).open("xb") as stream:
            stream.write(data)
    manifest = "".join(packages[name][1] + "  " + name + "\n" for name in sorted(packages))
    with (output / "SHA256SUMS").open("xb") as stream:
        stream.write(manifest.encode("ascii"))
    return sorted(packages)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--windows", required=True, type=Path)
    parser.add_argument("--macos", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        names = collect(args.version, args.windows, args.macos, args.output)
    except (OSError, ValueError) as error:
        parser.exit(1, str(error) + "\n")
    print("Checked release assets: " + ", ".join(names) + " and SHA256SUMS")


if __name__ == "__main__":
    main()
