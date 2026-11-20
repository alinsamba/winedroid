#!/usr/bin/env sh
set -eu

apk_path="${1:-${WINEDROID_BENCHMARK_APK:-}}"
if [ -z "$apk_path" ]; then
  echo "usage: scripts/profile_apk.sh /path/to/app.apk" >&2
  echo "or set WINEDROID_BENCHMARK_APK=/path/to/app.apk" >&2
  exit 64
fi

WINEDROID_TRACE="${WINEDROID_TRACE:-perf,runtime,launch,inspect,install,ui,render}" \
  cargo run -- "$apk_path"
