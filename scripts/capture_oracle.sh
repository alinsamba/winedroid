#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

APK="${1:?usage: capture_oracle.sh <apk> <package> <journey.auto> <out_dir>}"
PACKAGE="${2:?missing package name}"
JOURNEY="${3:?missing journey file}"
OUT_DIR="${4:?missing output directory}"

ADB="$HOME/Library/Android/sdk/platform-tools/adb"
EMULATOR="$HOME/Library/Android/sdk/emulator/emulator"
AVD="${WINEDROID_ORACLE_AVD:-$("$EMULATOR" -list-avds | head -1)}"
JOURNEY_ABS="$(cd "$(dirname "$JOURNEY")" && pwd)/$(basename "$JOURNEY")"

[ -f "$APK" ] || { echo "capture: APK not found: $APK" >&2; exit 1; }
[ -f "$JOURNEY" ] || { echo "capture: journey not found: $JOURNEY" >&2; exit 1; }

mkdir -p "$OUT_DIR/emulator" "$OUT_DIR/winedroid"

STARTED_EMULATOR=0
cleanup() {
  if [ "$STARTED_EMULATOR" = 1 ] && [ "${WINEDROID_ORACLE_KILL:-0}" = 1 ]; then
    "$ADB" emu kill >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if ! "$ADB" get-state >/dev/null 2>&1; then
  echo "capture: booting emulator $AVD headless"
  "$EMULATOR" -avd "$AVD" -no-window -no-audio -no-boot-anim -no-snapshot-save \
    >"$OUT_DIR/emulator-boot.log" 2>&1 &
  STARTED_EMULATOR=1
  "$ADB" wait-for-device shell \
    'while [ "$(getprop sys.boot_completed)" != "1" ]; do sleep 2; done'
  "$ADB" shell input keyevent KEYCODE_WAKEUP
  sleep 1
  "$ADB" shell wm dismiss-keyguard || true
  sleep 2
fi

echo "capture: installing $APK"
"$ADB" install -r "$APK" >/dev/null

ACTIVITY="$("$ADB" shell cmd package resolve-activity --brief \
  -c android.intent.category.LAUNCHER "$PACKAGE" | tail -1 | tr -d '\r')"
echo "capture: launching $ACTIVITY"
"$ADB" shell am force-stop "$PACKAGE"
"$ADB" shell am start -n "$ACTIVITY" >/dev/null
sleep 3

echo "capture: running journey on the emulator"
python3 scripts/emu_journey.py --artifacts "$OUT_DIR/emulator" "$JOURNEY_ABS"

echo "capture: running journey on winedroid"
cargo build --release --offline >/dev/null
BIN="$(pwd)/target/release/winedroid"
WINEDROID_WORK="$(mktemp -d "${TMPDIR:-/tmp}/winedroid-oracle.XXXXXX")"
(
  cd "$OUT_DIR/winedroid"
  env -u WINEDROID_TRACE -u WINEDROID_DEX_TRACE \
    WINEDROID_HEADLESS=1 \
    WINEDROID_PREFIX="$WINEDROID_WORK/prefix" \
    WINEDROID_AUTOMATION_SCRIPT="$JOURNEY_ABS" \
    WINEDROID_WINDOW_AUTOCLOSE_MS=1000 \
    "$BIN" "$APK" >"$WINEDROID_WORK/run.log" 2>&1
) || { cat "$WINEDROID_WORK/run.log" >&2; echo "capture: winedroid journey failed" >&2; exit 1; }
rm -rf "$WINEDROID_WORK"

STATUS=0
for expected in "$OUT_DIR"/emulator/*.xml; do
  [ -e "$expected" ] || continue
  name="$(basename "$expected")"
  actual="$OUT_DIR/winedroid/$name"
  if [ ! -f "$actual" ]; then
    echo "capture: winedroid produced no checkpoint $name" >&2
    STATUS=1
    continue
  fi
  echo "capture: diff $name"
  python3 scripts/diff_ui_dump.py "$expected" "$actual" || STATUS=1
done

if [ "$STATUS" = 0 ]; then
  echo "capture: OK — checkpoints match"
else
  echo "capture: divergence found — see report above; artifacts in $OUT_DIR"
fi
exit "$STATUS"
