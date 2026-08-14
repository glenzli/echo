#!/bin/sh

# Report or safely prune Echo-managed local build artifacts. It deliberately
# never touches unmarked /private/tmp candidates, catalogs, caches, or models.
set -eu

usage() {
    cat <<'EOF'
usage:
  ./scripts/prune_local_builds.sh [--apply] [--scratch] [--releases]
                                  [--keep-releases COUNT]
                                  [--older-than-hours HOURS]
                                  [--all-scratch] [--report-legacy-tmp]

Without --apply this command only reports candidates. It only removes managed
scratch directories containing Echo's marker and verified Debug releases.

--scratch                 Consider only managed scratch builds.
--releases                Consider only old Debug releases.
--keep-releases COUNT     Preserve the newest COUNT releases (default: 3).
--older-than-hours HOURS  Scratch age threshold (default: 24).
--all-scratch             Ignore the scratch age threshold, but still skip a
                          candidate whose recorded creator process is alive.
--report-legacy-tmp       Report unowned /private/tmp/echo-* directories;
                          they are never deleted by this command.
--apply                   Perform the reported safe removals.
EOF
}

apply=0
include_scratch=1
include_releases=1
report_legacy_tmp=0
keep_releases=3
older_than_hours=24
all_scratch=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --apply) apply=1 ;;
        --scratch) include_scratch=1; include_releases=0 ;;
        --releases) include_releases=1; include_scratch=0 ;;
        --keep-releases)
            shift
            keep_releases=${1:-}
            ;;
        --older-than-hours)
            shift
            older_than_hours=${1:-}
            ;;
        --all-scratch) all_scratch=1 ;;
        --report-legacy-tmp) report_legacy_tmp=1 ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; exit 64 ;;
    esac
    shift
done

case "$keep_releases" in
    ''|*[!0-9]*) echo "prune local builds: --keep-releases requires a non-negative integer" >&2; exit 64 ;;
esac
case "$older_than_hours" in
    ''|*[!0-9]*) echo "prune local builds: --older-than-hours requires a non-negative integer" >&2; exit 64 ;;
esac

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${ECHO_LOCAL_BUILD_ROOT:-"$repository_parent/.echo-local-build"}
scratch_root="$local_build_root/scratch"
release_root="$local_build_root/releases/debug"
current_link="$local_build_root/current-debug"
now_epoch=$(date +%s)
scratch_cutoff_epoch=$((now_epoch - older_than_hours * 3600))

action_word=would-remove
if [ "$apply" = "1" ]; then
    action_word=remove
fi

remove_directory() {
    candidate=$1
    reason=$2
    case "$candidate" in
        "$scratch_root"/desktop.*|"$release_root"/*) ;;
        *)
            echo "prune local builds: refusing an out-of-root removal target: $candidate" >&2
            exit 70
            ;;
    esac
    echo "$action_word: $candidate ($reason)"
    if [ "$apply" = "1" ]; then
        rm -rf "$candidate"
    fi
}

scratch_is_live() {
    candidate=$1
    marker="$candidate/.echo-scratch-build"
    creator_pid=$(sed -n 's/^pid=//p' "$marker" | head -n 1)
    case "$creator_pid" in
        ''|*[!0-9]*) return 1 ;;
    esac
    kill -0 "$creator_pid" 2>/dev/null
}

if [ "$include_scratch" = "1" ] && [ -d "$scratch_root" ]; then
    for candidate in "$scratch_root"/desktop.*; do
        [ -d "$candidate" ] || continue
        marker="$candidate/.echo-scratch-build"
        if ! grep -qx 'schema=echo-desktop-scratch-build-v1' "$marker" 2>/dev/null; then
            echo "skip: $candidate (not an Echo-managed scratch build)"
            continue
        fi
        if scratch_is_live "$candidate"; then
            echo "skip: $candidate (creator process is still alive)"
            continue
        fi
        modified_epoch=$(stat -f %m "$candidate")
        if [ "$all_scratch" != "1" ] && [ "$modified_epoch" -gt "$scratch_cutoff_epoch" ]; then
            echo "skip: $candidate (newer than ${older_than_hours}h)"
            continue
        fi
        remove_directory "$candidate" "inactive managed scratch build"
    done
fi

if [ "$include_releases" = "1" ] && [ -d "$release_root" ]; then
    if pgrep -x Echo >/dev/null 2>&1; then
        echo "skip: Debug release pruning (an Echo process is running)"
    else
        current_release=
        if [ -L "$current_link" ]; then
            current_release=$(CDPATH= cd -- "$current_link" && pwd -P)
        fi
        release_index=0
        for candidate in $(ls -1dt "$release_root"/*/ 2>/dev/null || true); do
            candidate=${candidate%/}
            [ -d "$candidate" ] || continue
            manifest="$candidate/build-manifest.txt"
            if ! grep -qx 'schema=echo-canonical-debug-build-v1' "$manifest" 2>/dev/null; then
                echo "skip: $candidate (unverified release directory)"
                continue
            fi
            release_index=$((release_index + 1))
            if [ "$candidate" = "$current_release" ] || [ "$release_index" -le "$keep_releases" ]; then
                echo "keep: $candidate"
                continue
            fi
            remove_directory "$candidate" "Debug release beyond retention of $keep_releases"
        done
    fi
fi

if [ "$report_legacy_tmp" = "1" ]; then
    echo "legacy temporary candidates are report-only and require explicit manual review:"
    for candidate in /private/tmp/echo-*; do
        [ -d "$candidate" ] || continue
        case "$candidate" in
            /private/tmp/echo-*) du -sh "$candidate" ;;
        esac
    done
fi
