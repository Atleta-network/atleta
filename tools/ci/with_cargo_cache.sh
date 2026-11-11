#!/bin/sh
set -eu
# BuildKit exposes this trusted runner configuration only for the current RUN.
if [ ! -r /run/secrets/cargo_cache_env ]; then
  exec "$@"
fi
set -a
# shellcheck disable=SC1091 # Supplied by the runner as a BuildKit secret.
. /run/secrets/cargo_cache_env
set +a
export RUSTC_WRAPPER=/usr/local/bin/sccache CARGO_INCREMENTAL=0
export SCCACHE_IGNORE_SERVER_IO_ERROR=1
# Keep statistics and daemon lifetime private to this compilation instruction.
cache_temporary="$(mktemp -d)"
export SCCACHE_SERVER_UDS="$cache_temporary/server.sock"
cleanup() {
  /usr/local/bin/sccache --show-stats 2>/dev/null | sed -n \
    -e '/^Compile requests /p' -e '/^Cache hits /p' \
    -e '/^Cache misses /p' -e '/^Cache errors /p' || :
  /usr/local/bin/sccache --stop-server >/dev/null 2>&1 || :
  rm -rf "$cache_temporary"
}
trap cleanup EXIT
# Preserve the actual compiler/test exit status, including genuine failures.
"$@"
