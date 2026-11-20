#!/usr/bin/env bash
set -euo pipefail

export WINEDROID_STRICT=1

cd "$(dirname "$0")/.."

OPENCALC_APK="${WINEDROID_OPENCALC_APK:-$HOME/com.darkempire78.opencalculator_53.apk}"
MEDTIMER_APK="${WINEDROID_MEDTIMER_APK:-$HOME/com.exner.tools.meditationtimer_10.apk}"
OPENNOTES_APK="${WINEDROID_OPENNOTES_APK:-$HOME/com.opennotes_14.apk}"
SMOKE_DEADLINE_SECONDS=120

WINEDROID_ENV_RESET=(
  -u WINEDROID_TRACE
  -u WINEDROID_DEX_TRACE
  -u WINEDROID_PREFIX
  -u WINEDROID_HEADLESS
  -u WINEDROID_AUTOMATION_SCRIPT
  -u WINEDROID_AUTOMATION_LOG
  -u WINEDROID_AUTOMATION_INTERACTIVE
  -u WINEDROID_WINDOW_AUTOCLOSE_MS
  -u WINEDROID_WINDOW_AUTOCLICK_TARGETS
  -u WINEDROID_WINDOW_AUTOCLICK_VIEW_ID
  -u WINEDROID_WINDOW_AUTOCLICK_VIEW_IDS
  -u WINEDROID_WINDOW_AUTOCLICK_DIALOG_BUTTON
  -u WINEDROID_WINDOW_NATIVE_CONTROLS
  -u WINEDROID_RENDER_DUMP_PNG
  -u WINEDROID_RENDER_SCROLL_Y
  -u WINEDROID_UI_DUMP
  -u WINEDROID_MISSING_PRIMITIVE_LOG
)

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/winedroid-validate.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "validate: FAILED — $1" >&2
  exit 1
}

echo "validate: comment citation checker self-test"
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_comment_citations.py >/dev/null || fail "comment citation checker self-test failed"

echo "validate: comment citation policy"
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_comment_citations.py --repository || fail "comment citation policy check failed"

for apk in "$OPENCALC_APK" "$MEDTIMER_APK" "$OPENNOTES_APK"; do
  [ -f "$apk" ] || fail "benchmark APK not found: $apk (override with WINEDROID_OPENCALC_APK / WINEDROID_MEDTIMER_APK / WINEDROID_OPENNOTES_APK)"
done

echo "validate: cargo fmt --check"
cargo fmt --check || fail "cargo fmt --check reported unformatted code"

echo "validate: browser page display smoke"
node --check web/app.js || fail "web/app.js has invalid JavaScript syntax"
node --check web/library_winedroid.js || fail "web/library_winedroid.js has invalid JavaScript syntax"
node --check scripts/test_web_runtime.mjs || fail "scripts/test_web_runtime.mjs has invalid JavaScript syntax"
node --check scripts/test_web_page.mjs || fail "scripts/test_web_page.mjs has invalid JavaScript syntax"
node scripts/test_web_page.mjs || fail "browser page display smoke failed"

echo "validate: cargo test --offline"
cargo test --offline -- --quiet || fail "test suite did not pass"

echo "validate: UI-dump differ self-test"
python3 scripts/test_diff_ui_dump.py >/dev/null || fail "scripts/diff_ui_dump.py self-test failed"

echo "validate: release build"
cargo build --release --offline || fail "release build failed"

BIN="$(pwd)/target/release/winedroid"

run_with_deadline() {
  local deadline="$1"
  shift
  "$@" &
  local pid=$!
  local waited=0
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$waited" -ge "$deadline" ]; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      return 124
    fi
    sleep 1
    waited=$((waited + 1))
  done
  wait "$pid"
}

run_smoke() {
  local name="$1"
  local smoke_apk="$2"
  local script="$WORK_DIR/$name.auto"
  printf '%s\n' "$3" >"$script"

  local status=0
  run_with_deadline "$SMOKE_DEADLINE_SECONDS" \
    env "${WINEDROID_ENV_RESET[@]}" \
    WINEDROID_HEADLESS=1 \
    WINEDROID_PREFIX="$WORK_DIR/prefix-$name" \
    WINEDROID_AUTOMATION_SCRIPT="$script" \
    WINEDROID_WINDOW_AUTOCLOSE_MS=25000 \
    "$BIN" "$smoke_apk" >"$WORK_DIR/$name.log" 2>&1 || status=$?

  if [ "$status" -ne 0 ]; then
    cat "$WORK_DIR/$name.log" >&2
    if [ "$status" -eq 124 ]; then
      fail "$name smoke hung past ${SMOKE_DEADLINE_SECONDS}s and was killed"
    fi
    fail "$name smoke exited nonzero"
  fi
}

echo "validate: Meditation Timer smoke (reaches Create form)"
run_smoke med "$MEDTIMER_APK" '0    waitidle 8000
0    tap 388:725
500  waittext "Create" 8000'

echo "validate: Open Calculator smoke (no title, menu, 7+8=15)"
run_smoke calc-math "$OPENCALC_APK" '0    waittext "AC" 8000
0    assert notext "Calculator"
0    assert notext "com.darkempire78"
300  tap 390:30
300  waittext "Settings" 4000
0    assert text "About"
0    assert text "Clear History"
300  tap 58:507
300  assert notext "Settings"
300  tap 58:406
300  tap 361:608
300  tap 159:406
300  tap 361:709
0    waitidle 3000
0    assert text "15"'

echo "validate: Open Calculator smoke (settings view opens)"
run_smoke calc-settings "$OPENCALC_APK" '0    waittext "AC" 8000
300  tap 390:30
300  waittext "Settings" 4000
300  tap 302:176
500  waittext "GENERAL" 5000'

echo "validate: Open Notes smoke (note list loads from SQLite)"
run_smoke notes "$OPENNOTES_APK" '0    waitidle 15000
0    waittext "New Note" 10000'

run_golden_journey() {
  local name="$1"
  local apk="$2"
  local journey="$3"
  local golden_dir="$4"
  local work_dir="$WORK_DIR/journey-$name"
  mkdir -p "$work_dir"

  local status=0
  (
    cd "$work_dir"
    run_with_deadline "$SMOKE_DEADLINE_SECONDS" \
      env "${WINEDROID_ENV_RESET[@]}" \
      WINEDROID_HEADLESS=1 \
      WINEDROID_PREFIX="$WORK_DIR/prefix-journey-$name" \
      WINEDROID_AUTOMATION_SCRIPT="$journey" \
      WINEDROID_WINDOW_AUTOCLOSE_MS=1000 \
      "$BIN" "$apk" >"$work_dir/run.log" 2>&1
  ) || status=$?

  if [ "$status" -ne 0 ]; then
    cat "$work_dir/run.log" >&2
    if [ "$status" -eq 124 ]; then
      fail "$name journey hung past ${SMOKE_DEADLINE_SECONDS}s and was killed"
    fi
    fail "$name journey exited nonzero"
  fi

  for golden in "$golden_dir"/*.xml; do
    case "$golden" in *.reference.xml) continue ;; esac
    local checkpoint
    checkpoint="$(basename "$golden")"
    if ! diff -u "$golden" "$work_dir/$checkpoint" >&2; then
      fail "$name journey checkpoint $checkpoint no longer matches its golden (see tests/goldens/*/NOTES.md to regenerate deliberately)"
    fi
  done
}

echo "validate: Open Calculator basic-math golden journey (UI dumps match frozen goldens)"
run_golden_journey opencalc-basic-math "$OPENCALC_APK" \
  "$(pwd)/tests/journeys/com.darkempire78.opencalculator/basic-math.auto" \
  "$(pwd)/tests/goldens/com.darkempire78.opencalculator/basic-math"

echo "validate: Open Calculator math-and-menu golden journey (menu + dialog checkpoints)"
run_golden_journey opencalc-math-and-menu "$OPENCALC_APK" \
  "$(pwd)/tests/journeys/com.darkempire78.opencalculator/math-and-menu.auto" \
  "$(pwd)/tests/goldens/com.darkempire78.opencalculator/math-and-menu"

echo "validate: Open Calculator menu-about golden journey (overflow menu reopens)"
run_golden_journey opencalc-menu-about "$OPENCALC_APK" \
  "$(pwd)/tests/journeys/com.darkempire78.opencalculator/menu-about.auto" \
  "$(pwd)/tests/goldens/com.darkempire78.opencalculator/menu-about"

echo "validate: OK"
