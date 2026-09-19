#!/bin/sh

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
repository_parent=$(dirname -- "$repository_root")
local_build_root=${ECHO_LOCAL_BUILD_ROOT:-"$repository_parent/.echo-local-build"}
canonical_app="$local_build_root/current-debug/Echo.app"
echo_executable="$canonical_app/Contents/MacOS/Echo"
catalog=${ECHO_DEBUG_CATALOG_PATH:-"$repository_root/catalogs/demo.sqlite"}
cache_root=${ECHO_DEBUG_CACHE_ROOT:-"$repository_root/cache"}
debug_log_root=${ECHO_DEBUG_LOG_ROOT:-"$local_build_root/logs"}
debug_log="$debug_log_root/echo-debug.log"

usage() {
    cat <<'EOF'
usage:
  ./scripts/run_debug.sh [catalog.sqlite [cache-root]]
  ./scripts/run_debug.sh --foreground [catalog.sqlite [cache-root]]
  ./scripts/run_debug.sh [--foreground] --edit [audio files ...]
  ./scripts/run_debug.sh [--foreground] --project /absolute/project.echo
  ./scripts/run_debug.sh --check

By default Echo starts in the background from the canonical debug build and
this script returns immediately. Use --foreground to keep Echo attached to the
terminal. Without explicit data paths, the ignored catalogs/demo.sqlite and
cache directories in the repository are reused across launches.
EOF
}

if [ ! -x "$echo_executable" ]; then
    echo "Echo has no promoted canonical debug build." >&2
    echo "Expected: $canonical_app" >&2
    echo "Run ./scripts/build_and_promote_debug.sh first." >&2
    exit 69
fi
if [ -f "$canonical_app/Contents/Info.plist" ] && command -v plutil >/dev/null 2>&1; then
    plutil -lint "$canonical_app/Contents/Info.plist" >/dev/null
fi

if [ "${1:-}" = "--check" ]; then
    [ "$#" -eq 1 ] || { echo "--check does not accept extra arguments" >&2; exit 64; }
    echo "canonical debug app: $canonical_app"
    echo "default catalog: $catalog"
    echo "default cache: $cache_root"
    echo "log: $debug_log"
    exit 0
fi

foreground=false
case "${1:-}" in
    --foreground)
        foreground=true
        shift
        ;;
    -h|--help)
        usage
        exit 0
        ;;
esac

case "${1:-}" in
    --edit|--project|--resume-editor)
        if [ "$foreground" = true ]; then
            exec "$echo_executable" "$@"
        fi
        mkdir -p "$debug_log_root"
        nohup "$echo_executable" "$@" >>"$debug_log" 2>&1 </dev/null &
        echo "Echo independent editor started (pid $!)."
        echo "log: $debug_log"
        exit 0
        ;;
esac

[ "$#" -le 2 ] || { usage >&2; exit 64; }
if [ "$#" -ge 1 ]; then
    catalog=$1
fi
if [ "$#" -eq 2 ]; then
    cache_root=$2
fi

mkdir -p "$(dirname -- "$catalog")" "$cache_root"
if [ "$foreground" = true ]; then
    exec "$echo_executable" "$catalog" "$cache_root"
fi

mkdir -p "$debug_log_root"
nohup "$echo_executable" "$catalog" "$cache_root" >>"$debug_log" 2>&1 </dev/null &
echo_pid=$!
echo "Echo started in the background (pid $echo_pid)."
echo "app: $canonical_app"
echo "catalog: $catalog"
echo "log: $debug_log"
