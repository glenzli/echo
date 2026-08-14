#!/bin/sh

# Build a disposable desktop candidate outside the worktree. This is for
# focused validation; use build_and_promote_debug.sh for the runnable Debug app.
set -eu

usage() {
    cat <<'EOF'
usage:
  ./scripts/build_desktop_scratch.sh [--keep]

Builds Echo.app in a managed scratch directory outside the worktree. The
directory is removed when the command exits, including after a failed build.
Pass --keep (or ECHO_KEEP_BUILD=1) only when you need to inspect the candidate
afterward; stale kept candidates can be listed and pruned by
scripts/prune_local_builds.sh.
EOF
}

keep_build=${ECHO_KEEP_BUILD:-0}
case "${1:-}" in
    "") ;;
    --keep) keep_build=1 ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; exit 64 ;;
esac

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${ECHO_LOCAL_BUILD_ROOT:-"$repository_parent/.echo-local-build"}
scratch_root="$local_build_root/scratch"

mkdir -p "$scratch_root"
scratch_directory=$(mktemp -d "$scratch_root/desktop.XXXXXX")
marker="$scratch_directory/.echo-scratch-build"

cleanup() {
    case "$scratch_directory" in
        "$scratch_root"/desktop.*)
            if [ "$keep_build" != "1" ] && [ -f "$marker" ]; then
                rm -rf "$scratch_directory"
            fi
            ;;
    esac
}
trap cleanup EXIT HUP INT TERM

{
    echo "schema=echo-desktop-scratch-build-v1"
    echo "pid=$$"
    echo "source_root=$repository_root"
} >"$marker"

cmake --preset desktop-dev -B "$scratch_directory"
cmake --build "$scratch_directory" --target echo-desktop --parallel 6

candidate_app="$scratch_directory/apps/desktop/Echo.app"
candidate_executable="$candidate_app/Contents/MacOS/Echo"
if [ ! -x "$candidate_executable" ]; then
    echo "desktop scratch build: Echo.app was not produced" >&2
    exit 69
fi

if [ "$keep_build" = "1" ]; then
    echo "kept scratch app: $candidate_app"
else
    echo "validated scratch app; removing: $scratch_directory"
fi
