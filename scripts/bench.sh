#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

OPENNOTES_APK="${WINEDROID_OPENNOTES_APK:-$HOME/com.opennotes_14.apk}"
MEDTIMER_APK="${WINEDROID_MEDTIMER_APK:-$HOME/com.exner.tools.meditationtimer_10.apk}"
BENCH_RUNS="${BENCH_RUNS:-3}"
BENCH_DEADLINE_SECONDS=180
BASELINE="benchmarks/baseline.json"
RESULTS="benchmarks/results.json"

MODE="report"
case "${1:-}" in
  --update-baseline) MODE="update" ;;
  --check) MODE="check" ;;
  "") ;;
  *) echo "usage: scripts/bench.sh [--update-baseline|--check]" >&2; exit 64 ;;
esac

WINEDROID_ENV_RESET=(
  -u WINEDROID_TRACE
  -u WINEDROID_DEX_TRACE
  -u WINEDROID_METHOD_TRACE
  -u WINEDROID_PREFIX
  -u WINEDROID_HEADLESS
  -u WINEDROID_AUTOMATION_SCRIPT
  -u WINEDROID_AUTOMATION_LOG
  -u WINEDROID_AUTOMATION_INTERACTIVE
  -u WINEDROID_WINDOW_AUTOCLOSE_MS
  -u WINEDROID_RENDER_DUMP_PNG
  -u WINEDROID_UI_DUMP
  -u WINEDROID_MISSING_PRIMITIVE_LOG
)

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/winedroid-bench.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "bench: FAILED — $1" >&2
  exit 1
}

for apk in "$OPENNOTES_APK" "$MEDTIMER_APK"; do
  [ -f "$apk" ] || fail "benchmark APK not found: $apk"
done

echo "bench: release build"
cargo build --release --offline >/dev/null 2>&1 || fail "release build failed"
BIN="target/release/winedroid"

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

run_scenario() {
  local name="$1"
  local apk="$2"
  local script="$WORK_DIR/$name.auto"
  printf '%s\n' "$3" >"$script"

  local run
  for run in $(seq 1 "$BENCH_RUNS"); do
    local status=0
    rm -rf "$WORK_DIR/prefix-$name"
    local started_ms ended_ms
    started_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
    run_with_deadline "$BENCH_DEADLINE_SECONDS" \
      env "${WINEDROID_ENV_RESET[@]}" \
      WINEDROID_HEADLESS=1 \
      WINEDROID_PREFIX="$WORK_DIR/prefix-$name" \
      WINEDROID_AUTOMATION_SCRIPT="$script" \
      WINEDROID_AUTOMATION_LOG="$WORK_DIR/$name.$run.csv" \
      WINEDROID_WINDOW_AUTOCLOSE_MS=2000 \
      "$BIN" "$apk" >"$WORK_DIR/$name.$run.log" 2>&1 || status=$?
    ended_ms=$(python3 -c 'import time; print(int(time.time()*1000))')
    echo $((ended_ms - started_ms)) >"$WORK_DIR/$name.$run.wall"
    if [ "$status" -ne 0 ]; then
      cat "$WORK_DIR/$name.$run.log" >&2
      fail "$name run $run exited nonzero"
    fi
  done
}

run_scenario opennotes "$OPENNOTES_APK" '0    waitidle 15000
0    waittext "New Note" 8000
300  tap 339:716
0    waitidle 8000
300  tap 40:115
300  text "a"
0    waitidle 8000'

run_scenario medtimer "$MEDTIMER_APK" '0    waitidle 15000
0    waittext "Meditation Timer" 8000'

python3 - "$WORK_DIR" "$BENCH_RUNS" "$MODE" "$BASELINE" "$RESULTS" <<'PYEOF'
import csv, json, statistics, sys
from pathlib import Path

work_dir, runs, mode, baseline_path, results_path = sys.argv[1:6]
runs = int(runs)

METRICS = {
    "opennotes_wall_ms": ("opennotes", []),
    "opennotes_editor_open_ms": ("opennotes", [3, 4]),
    "opennotes_keystroke_ms": ("opennotes", [6, 7]),
    "medtimer_wall_ms": ("medtimer", []),
}

def step_latencies(path):
    out = {}
    with open(path) as fh:
        for row in csv.DictReader(fh):
            out[int(row["step"])] = int(row["latency_ms"])
    return out

results = {}
for metric, (scenario, steps) in METRICS.items():
    samples = []
    for run in range(1, runs + 1):
        if steps:
            latencies = step_latencies(Path(work_dir) / f"{scenario}.{run}.csv")
            samples.append(sum(latencies[s] for s in steps))
        else:
            samples.append(int((Path(work_dir) / f"{scenario}.{run}.wall").read_text()))
    results[metric] = {
        "median_ms": int(statistics.median(samples)),
        "samples_ms": samples,
    }

Path(results_path).parent.mkdir(parents=True, exist_ok=True)
Path(results_path).write_text(json.dumps(results, indent=2) + "\n")

baseline = {}
if Path(baseline_path).exists():
    baseline = json.loads(Path(baseline_path).read_text())

print(f"bench: medians over {runs} runs")
regressed = []
for metric, data in results.items():
    line = f"bench:   {metric:34} {data['median_ms']:>7} ms"
    base = baseline.get(metric, {}).get("median_ms")
    if base:
        ratio = data["median_ms"] / base if base else 0
        line += f"   baseline {base} ms ({ratio:.2f}x)"
        if ratio > 1.5:
            regressed.append(metric)
    print(line)

if mode == "update":
    Path(baseline_path).write_text(json.dumps(results, indent=2) + "\n")
    print(f"bench: baseline updated -> {baseline_path}")
elif regressed:
    print(f"bench: REGRESSED >1.5x: {', '.join(regressed)}")
    if mode == "check":
        sys.exit(1)
PYEOF
