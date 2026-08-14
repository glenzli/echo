#!/bin/sh

# Build the complete user-launchable Echo debug product, validate its real
# packaged startup path, and atomically advance the stable current-debug app.
set -eu

usage() {
    cat <<'EOF'
usage:
  ./scripts/build_and_promote_debug.sh [validation-label]
  ./scripts/build_and_promote_debug.sh --check

Builds Echo.app in one stable external candidate directory, verifies an
offscreen startup, then atomically promotes the bundle to current-debug.

Optional overrides:
  ECHO_CANONICAL_DEBUG_BUILD_DIR         External CMake candidate directory.
  ECHO_CANONICAL_DEBUG_CARGO_TARGET_DIR  External Cargo target for the i18n gate.
  ECHO_LOCAL_BUILD_ROOT                  Root containing releases and current-debug.
EOF
}

fail() {
    echo "canonical debug build: $*" >&2
    exit 69
}

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${ECHO_LOCAL_BUILD_ROOT:-"$repository_parent/.echo-local-build"}
build_directory=${ECHO_CANONICAL_DEBUG_BUILD_DIR:-"$local_build_root/canonical-debug-build"}
cargo_target_directory=${ECHO_CANONICAL_DEBUG_CARGO_TARGET_DIR:-"$repository_parent/.echo-local-target/canonical-debug-xtask"}
candidate_app="$build_directory/apps/desktop/Echo.app"
candidate_executable="$candidate_app/Contents/MacOS/Echo"

case "$build_directory" in
    /*) ;;
    *) fail "ECHO_CANONICAL_DEBUG_BUILD_DIR must be an absolute path" ;;
esac
case "$cargo_target_directory" in
    /*) ;;
    *) fail "ECHO_CANONICAL_DEBUG_CARGO_TARGET_DIR must be an absolute path" ;;
esac
case "$build_directory" in
    "$repository_root"|"$repository_root"/*) fail "candidate build directory must stay outside the source tree" ;;
esac
case "$cargo_target_directory" in
    "$repository_root"|"$repository_root"/*) fail "Cargo target directory must stay outside the source tree" ;;
esac

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    usage
    exit 0
fi
if [ "${1:-}" = "--check" ]; then
    [ "$#" -eq 1 ] || fail "--check does not accept a validation label"
    echo "candidate build directory: $build_directory"
    echo "candidate app: $candidate_app"
    echo "canonical app: $local_build_root/current-debug/Echo.app"
    exit 0
fi
[ "$#" -le 1 ] || { usage >&2; exit 64; }

validation_label=${1:-"canonical-debug-fast-$(git -C "$repository_root" rev-parse --short HEAD)"}

(
    cd "$repository_root"
    CARGO_TARGET_DIR="$cargo_target_directory" \
        cargo run --package xtask -- desktop-i18n-check
)
cmake --preset desktop-dev -B "$build_directory"
cmake --build "$build_directory" --target echo-desktop --parallel 6

smoke_root=$(mktemp -d "${TMPDIR:-/tmp}/echo-canonical-debug.XXXXXX")
cleanup() {
    case "$smoke_root" in
        "${TMPDIR:-/tmp}"/echo-canonical-debug.*) rm -rf "$smoke_root" ;;
    esac
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$smoke_root/cache"
QT_QPA_PLATFORM=offscreen \
ECHO_DEBUG_SCREENSHOT="$smoke_root/startup.png" \
    "$candidate_executable" "$smoke_root/catalog.sqlite" "$smoke_root/cache"
[ -s "$smoke_root/startup.png" ] || fail "packaged offscreen startup produced no screenshot"

"$repository_root/scripts/promote_debug_build.sh" "$candidate_app" "$validation_label"
"$repository_root/scripts/run_debug.sh" --check

# Promotion is successful even if maintenance is unavailable. The pruning
# helper is conservative: it preserves the current release and skips every
# release while Echo is running.
if ! "$repository_root/scripts/prune_local_builds.sh" --apply --scratch --older-than-hours 24; then
    echo "canonical debug build: scratch maintenance was skipped" >&2
fi
if ! "$repository_root/scripts/prune_local_builds.sh" --apply --releases --keep-releases "${ECHO_DEBUG_RELEASE_KEEP:-3}"; then
    echo "canonical debug build: release maintenance was skipped" >&2
fi
