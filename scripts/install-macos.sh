#!/bin/bash
# Install only the original CLI. No QQ, credentials, shell profiles or system protections are changed.
set -euo pipefail
fail() { echo "qq-cli installer: $*" >&2; exit 1; }
usage() {
    echo 'Usage: install-macos.sh [--archive ZIP --checksums FILE] [--directory DIR] [--bin-directory DIR] [--uninstall]'
}
[[ $(uname -s) == Darwin && $(uname -m) == arm64 ]] || fail 'Native Apple Silicon macOS is required.'
install_directory="${HOME}/.local/lib/qq-cli"
bin_directory="${HOME}/.local/bin"
archive=''
checksums=''
uninstall=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --archive|--checksums|--directory|--bin-directory)
            [[ $# -ge 2 && -n $2 ]] || fail "Missing value for $1."
            case "$1" in
                --archive) archive=$2;;
                --checksums) checksums=$2;;
                --directory) install_directory=$2;;
                --bin-directory) bin_directory=$2;;
            esac
            shift 2;;
        --uninstall) uninstall=true; shift;;
        --help) usage; exit 0;;
        *) usage >&2; fail 'Unknown installer option.';;
    esac
done
[[ -z $archive && -z $checksums || -n $archive && -n $checksums ]] || fail 'Supply both --archive and --checksums.'
[[ $uninstall == false || -z $archive ]] || fail 'Uninstall does not accept an archive.'

# Reject path aliases before creating anything; /tmp callers can use /private/tmp.
absolute_directory() {
    local path=$1 current
    [[ $path == /* ]] || path="$PWD/$path"
    path=${path%/}
    [[ -n $path && $path != *$'\n'* && $path != *$'\r'* ]] || fail 'Invalid installation directory.'
    case "$path/" in *'/../'*|*'/./'*) fail 'Installation directories must not contain dot components.';; esac
    current=$path
    while [[ -n $current ]]; do
        [[ ! -L $current ]] || fail 'Installation directories must not contain symbolic links.'
        current=${current%/*}
    done
    printf '%s\n' "$path"
}
install_directory=$(absolute_directory "$install_directory")
bin_directory=$(absolute_directory "$bin_directory")
[[ $install_directory != "$bin_directory" ]] || fail 'Use separate installation and command directories.'
target="$bin_directory/qq-cli"
marker="$install_directory/.qq-cli-install-v1"
lock=''
workspace=''
stage=''
link_stage=''
cleanup() {
    [[ -z $link_stage ]] || rm -f -- "$link_stage"
    [[ -z $stage ]] || rm -rf -- "$stage"
    [[ -z $workspace ]] || rm -rf -- "$workspace"
    [[ -z $lock ]] || rmdir -- "$lock"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
managed_target() {
    local destination
    [[ -L $target ]] || return 1
    destination=$(readlink "$target")
    [[ $destination == "$install_directory/versions/"*'/qq-cli' ]] || return 1
    destination=${destination#"$install_directory/versions/"}
    [[ $destination =~ ^[a-f0-9]{64}/qq-cli$ ]]
}
validate_existing() {
    [[ ! -L $marker && -f $marker && $(cat "$marker") == 'qq-cli installer v1' ]] || fail 'Installation ownership marker is missing or invalid.'
    [[ ! -e $target && ! -L $target ]] || managed_target || fail 'Existing command is not owned by this installation.'
}
private_directory() {
    [[ ! -L $1 && -d $1 && $(stat -f '%u' "$1") == $(id -u) ]] || fail 'Installer directory is not owned by the current user.'
    local mode
    mode=$(stat -f '%Lp' "$1")
    (( (8#$mode & 022) == 0 )) || fail 'Installer directory must not be writable by other users.'
}

if [[ $uninstall == true ]]; then
    validate_existing
    private_directory "$install_directory"
    mkdir "$install_directory/.install-lock" || fail 'Another installer is active; retain its lock until its outcome is known.'
    lock="$install_directory/.install-lock"
    validate_existing
    if [[ -L $target ]]; then rm -- "$target"; fi
    echo 'Removed the qq-cli command. Prior CLI versions, QQ data and Keychain credentials are preserved.'
    exit 0
fi
workspace=$(mktemp -d "${TMPDIR:-/tmp}/qq-cli-install.XXXXXXXX")
download_release() {
    curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 15 --max-time 60 --max-filesize 1048576 \
        -H 'Accept: application/vnd.github+json' -H 'User-Agent: qq-cli-installer' \
        https://api.github.com/repos/noctisynth/qq-cli/releases/latest > "$workspace/release.json" \
        || fail 'Cannot retrieve the latest stable release; check network/API limits.'
    # JXA is bundled with macOS; parsing JSON does not require Python, Rust or Git.
    /usr/bin/osascript -l JavaScript - "$workspace/release.json" > "$workspace/assets" <<'JXA'
ObjC.import('Foundation');
function run(args) {
    var raw = $.NSString.stringWithContentsOfFileEncodingError(args[0], $.NSUTF8StringEncoding, null);
    var release = JSON.parse(ObjC.unwrap(raw));
    if (release.draft || release.prerelease) throw Error('Expected a published stable release.');
    var archives = release.assets.filter(function(a) { return /^qq-cli-v\d+\.\d+\.\d+-macos-aarch64\.zip$/.test(a.name); });
    var sums = release.assets.filter(function(a) { return a.name === 'SHA256SUMS'; });
    if (archives.length !== 1 || sums.length !== 1) throw Error('This release must contain exactly one macOS arm64 archive and SHA256SUMS.');
    var selected = [archives[0], sums[0]];
    selected.forEach(function(a) {
        if (!/^https:\/\/github\.com\/noctisynth\/qq-cli\/releases\/download\/[A-Za-z0-9._+/-]+$/.test(a.browser_download_url)) throw Error('Unexpected release asset origin.');
    });
    return [archives[0].name, archives[0].browser_download_url, sums[0].browser_download_url].join('\n');
}
JXA
    [[ $(wc -l < "$workspace/assets" | tr -d ' ') == 3 ]] || fail 'Invalid release metadata.'
    asset_name=$(sed -n '1p' "$workspace/assets")
    archive="$workspace/$asset_name"
    checksums="$workspace/SHA256SUMS"
    curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 15 --max-time 300 --max-filesize 268435456 \
        "$(sed -n '2p' "$workspace/assets")" -o "$archive"
    curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 15 --max-time 60 --max-filesize 1048576 \
        "$(sed -n '3p' "$workspace/assets")" -o "$checksums"

}
if [[ -z $archive ]]; then download_release; fi
verify_archive() {
    [[ -f $archive && ! -L $archive && -f $checksums && ! -L $checksums ]] || fail 'Archive and checksums must be regular files.'
    asset_name=$(basename "$archive")
    [[ $asset_name =~ ^qq-cli-v[0-9]+\.[0-9]+\.[0-9]+([-+][A-Za-z0-9.+-]+)?-macos-aarch64\.zip$ ]] || fail 'Expected a macOS arm64 release archive name.'
    # Verify and extract the same private copy. Preserve source attributes using
    # the system copier instead of reopening a mutable supplied archive later.
    if [[ $archive != "$workspace/$asset_name" ]]; then
        /usr/bin/ditto "$archive" "$workspace/$asset_name"
        archive="$workspace/$asset_name"
    fi
    digest=$(shasum -a 256 "$archive" | cut -d ' ' -f 1)
    expected=$(awk -v name="$asset_name" 'NF == 2 && $2 == name {print $1}' "$checksums")
    [[ $expected =~ ^[a-fA-F0-9]{64}$ && $(printf '%s' "$expected" | tr 'A-F' 'a-f') == "$digest" ]] || fail 'Missing, duplicate or mismatched SHA256; existing installation was not changed.'
    cat > "$workspace/expected" <<'NAMES'
LICENSE-APACHE
LICENSE-MIT
README.md
qq-cli
skills/qq-cli/SKILL.md
skills/qq-cli/references/discover.md
skills/qq-cli/references/events.md
skills/qq-cli/references/media.md
skills/qq-cli/references/read.md
skills/qq-cli/references/runtime-macos.md
skills/qq-cli/references/runtime.md
skills/qq-cli/references/send.md
skills/qq-cli/references/shared.md
NAMES
    # Retain the complete earlier layout while main's installer also supports
    # the expanded task guides. Partial or unreviewed layouts still fail closed.
    cp "$workspace/expected" "$workspace/legacy-expected"
    cat >> "$workspace/expected" <<'TASK_NAMES'
skills/qq-cli/references/account.md
skills/qq-cli/references/assets.md
skills/qq-cli/references/contact.md
skills/qq-cli/references/files.md
skills/qq-cli/references/group.md
TASK_NAMES
    LC_ALL=C sort "$workspace/expected" > "$workspace/current-expected"
    unzip -Z -1 "$archive" | LC_ALL=C sort > "$workspace/actual"
    if diff -q "$workspace/current-expected" "$workspace/actual" >/dev/null; then
        cp "$workspace/current-expected" "$workspace/expected"
    elif diff -q "$workspace/legacy-expected" "$workspace/actual" >/dev/null; then
        cp "$workspace/legacy-expected" "$workspace/expected"
    else
        fail 'Archive entries differ from the original CLI distribution allowlist.'
    fi
}
verify_archive
publish_archive() {
    if [[ -e $install_directory ]]; then
        validate_existing
    else
        (umask 077; mkdir -p "$install_directory")
        printf '%s\n' 'qq-cli installer v1' > "$marker"
    fi
    private_directory "$install_directory"
    [[ ! -L $install_directory/versions ]] || fail 'Versions directory must not be a link.'
    (umask 077; mkdir -p "$install_directory/versions" "$bin_directory")
    private_directory "$install_directory/versions"
    private_directory "$bin_directory"
    mkdir "$install_directory/.install-lock" || fail 'Another installer is active; retain its lock until its outcome is known.'
    lock="$install_directory/.install-lock"
    validate_existing
    stage=$(mktemp -d "$install_directory/.stage.XXXXXXXX")
    # System ditto retains archive quarantine handling; never clear attributes or bypass Gatekeeper.
    /usr/bin/ditto -x -k "$archive" "$stage"
    [[ -z $(find "$stage" -type l -print -quit) ]] || fail 'Distribution links are not allowed.'
    while IFS= read -r name; do
        [[ -f $stage/$name ]] || fail 'Missing distribution file.'
    done < "$workspace/expected"
    header=$(od -An -tx1 -N16 "$stage/qq-cli" | tr -d ' \n')
    [[ $header == cffaedfe0c000001????????02000000 ]] || fail 'Expected a thin arm64 Mach-O executable.'
    [[ -x $stage/qq-cli ]] || fail 'Archive did not preserve CLI execution permissions.'
    printf '%s\n' "$digest" > "$stage/.archive-sha256"
    slot="$install_directory/versions/$digest"
    if [[ -e $slot || -L $slot ]]; then
        [[ ! -L $slot && -d $slot && ! -L $slot/.archive-sha256 && $(cat "$slot/.archive-sha256") == "$digest" ]] || fail 'Existing version slot is not authenticated.'
        [[ -z $(find "$slot" -type l -print -quit) ]] || fail 'Installed version contains a link; preserved without overwrite.'
        while IFS= read -r name; do
            [[ ! -L $slot/$name && -f $slot/$name ]] && cmp -s "$stage/$name" "$slot/$name" || fail 'Installed version changed; preserved without overwrite.'
        done < "$workspace/expected"
        [[ -x $slot/qq-cli ]] || fail 'Installed execution permissions changed.'
    else
        mv -- "$stage" "$slot"
        stage=''
    fi
    validate_existing
    previous=''
    if [[ -L $target ]]; then previous=$(readlink "$target"); fi
    link_stage="$bin_directory/.qq-cli-link-$$-$RANDOM"
    ln -s "$slot/qq-cli" "$link_stage"
    mv -fh -- "$link_stage" "$target"
    link_stage=''
    printf 'Installed command: %s\n' "$target"
    if [[ -n $previous && $previous != "$slot/qq-cli" ]]; then
        printf 'Previous CLI retained for recovery: %s\n' "$previous"
    fi
    case ":$PATH:" in *":$bin_directory:"*) ;; *) printf 'Add to your shell PATH: export PATH=%q:"$PATH"\n' "$bin_directory";; esac
    echo 'Keychain access is not migrated or weakened. If the new CLI cannot access existing credentials, retain both versions and use the previous CLI while resolving its signing identity.'
}
publish_archive
