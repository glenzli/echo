#!/bin/sh

set -eu

usage() {
    echo "usage: $0 /absolute/path/to/Echo.app validation-label" >&2
    exit 64
}

candidate_app=${1:-}
validation_label=${2:-}
if [ -z "$candidate_app" ] || [ -z "$validation_label" ]; then
    usage
fi
if [ "${candidate_app#/}" = "$candidate_app" ]; then
    echo "promote debug build: candidate app path must be absolute" >&2
    exit 64
fi

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${ECHO_LOCAL_BUILD_ROOT:-"$repository_parent/.echo-local-build"}
release_root="$local_build_root/releases/debug"
current_link="$local_build_root/current-debug"
promotion_lock="$local_build_root/.promote-debug-build.lock"
promotion_lock_helper="$script_directory/acquire_debug_promotion_lock.py"
echo_executable="$candidate_app/Contents/MacOS/Echo"

if [ ! -d "$candidate_app" ] || [ ! -x "$echo_executable" ]; then
    echo "promote debug build: candidate does not contain an executable Echo app" >&2
    exit 65
fi
if [ ! -r "$candidate_app/Contents/Info.plist" ]; then
    echo "promote debug build: candidate does not contain Echo bundle metadata" >&2
    exit 65
fi
if command -v plutil >/dev/null 2>&1; then
    plutil -lint "$candidate_app/Contents/Info.plist" >/dev/null
fi

incoming_release=
next_link=
promotion_lock_token=
cleanup() {
    if [ -n "$next_link" ] && [ -L "$next_link" ]; then
        rm "$next_link"
    fi
    case "$incoming_release" in
        "$release_root"/.incoming-*)
            if [ -d "$incoming_release" ]; then
                rm -rf "$incoming_release"
            fi
            ;;
    esac
    if [ -n "$promotion_lock_token" ]; then
        python3 "$promotion_lock_helper" release "$promotion_lock" \
            --token "$promotion_lock_token" || true
    fi
}

if ! mkdir -p "$local_build_root"; then
    echo "promote debug build: cannot create local build root: $local_build_root" >&2
    echo "promote debug build: this is an environment failure, not lock contention" >&2
    exit 77
fi
promotion_steward=${ECHO_CANONICAL_DEBUG_STEWARD:-"promotion-shell-$$"}
promotion_lock_token=$(
    python3 "$promotion_lock_helper" acquire "$promotion_lock" \
        --owner "$promotion_steward"
)
trap cleanup EXIT HUP INT TERM

if ! mkdir -p "$release_root"; then
    echo "promote debug build: cannot create release root: $release_root" >&2
    exit 77
fi
if [ -e "$current_link" ] && [ ! -L "$current_link" ]; then
    echo "promote debug build: canonical entry exists but is not a symlink: $current_link" >&2
    exit 73
fi

revision=$(git -C "$repository_root" rev-parse --short=12 HEAD)
promoted_at=$(date -u '+%Y%m%dT%H%M%SZ')
release_id="$revision-$promoted_at"
release_directory="$release_root/$release_id"
incoming_release="$release_root/.incoming-$release_id-$$"
if [ -e "$release_directory" ]; then
    echo "promote debug build: release already exists: $release_directory" >&2
    exit 73
fi

mkdir "$incoming_release"
ditto "$candidate_app" "$incoming_release/Echo.app"

echo_digest=$(shasum -a 256 "$echo_executable" | awk '{print $1}')
copied_executable="$incoming_release/Echo.app/Contents/MacOS/Echo"
copied_echo_digest=$(shasum -a 256 "$copied_executable" | awk '{print $1}')
if [ "$echo_digest" != "$copied_echo_digest" ]; then
    echo "promote debug build: copied application digest verification failed" >&2
    exit 74
fi

worktree_digest=$(
    git -C "$repository_root" status --porcelain=v1 --untracked-files=all |
        shasum -a 256 |
        awk '{print $1}'
)
{
    echo "schema=echo-canonical-debug-build-v1"
    echo "revision=$revision"
    echo "promoted_at_utc=$promoted_at"
    echo "validation=$validation_label"
    echo "source_app=$candidate_app"
    echo "worktree_status_sha256=$worktree_digest"
    echo "echo_executable_sha256=$echo_digest"
} >"$incoming_release/build-manifest.txt"

mv "$incoming_release" "$release_directory"
incoming_release=

next_link="$local_build_root/.current-debug-$release_id-$$"
ln -s "releases/debug/$release_id" "$next_link"
mv -fh "$next_link" "$current_link"
next_link=

canonical_app="$current_link/Echo.app"
if [ ! -x "$canonical_app/Contents/MacOS/Echo" ] || \
    [ ! -r "$canonical_app/Contents/Info.plist" ]; then
    echo "promote debug build: canonical link verification failed" >&2
    exit 74
fi

echo "canonical debug app: $canonical_app"
echo "run: $repository_root/scripts/run_debug.sh"
