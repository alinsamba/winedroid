#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

OPENNOTES_APK="${WINEDROID_OPENNOTES_APK:-$HOME/com.opennotes_14.apk}"
RUNS="${BENCH_RUNS:-7}"
MODE="headline"
FLOW_FILTER="all"
WORK_DIR="${BENCH_WORK_DIR:-}"
ARTIFACT_ROOT="${BENCH_ARTIFACT_ROOT:-benchmarks/open-notes/artifacts}"
ARTIFACT_DIR="${BENCH_ARTIFACT_DIR:-}"
RESULTS="benchmarks/open-notes/results.latest.json"
ORIGINAL_ARGS=("$@")

usage() {
  echo "usage: scripts/bench_open_notes_usage.sh [--runs N] [--flow NAME] [--opcode|--trace] [--results PATH] [--artifact-dir PATH]" >&2
  exit 64
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --runs)
      [ "$#" -ge 2 ] || usage
      RUNS="$2"
      shift 2
      ;;
    --flow)
      [ "$#" -ge 2 ] || usage
      FLOW_FILTER="$2"
      shift 2
      ;;
    --opcode)
      MODE="opcode"
      shift
      ;;
    --trace)
      MODE="trace"
      shift
      ;;
    --results)
      [ "$#" -ge 2 ] || usage
      RESULTS="$2"
      shift 2
      ;;
    --artifact-dir)
      [ "$#" -ge 2 ] || usage
      ARTIFACT_DIR="$2"
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

[ -f "$OPENNOTES_APK" ] || {
  echo "Open Notes APK not found: $OPENNOTES_APK" >&2
  exit 1
}

case "$RUNS" in
  ''|*[!0-9]*)
    echo "runs must be a positive integer" >&2
    exit 64
    ;;
esac

[ "$RUNS" -gt 0 ] || {
  echo "runs must be greater than zero" >&2
  exit 64
}

flow_names=(
  editor-open
  focused-control
  keystroke
  note-create
  sustained-typing
)

selected_flows=()
for flow in "${flow_names[@]}"; do
  if [ "$FLOW_FILTER" = all ] || [ "$FLOW_FILTER" = "$flow" ]; then
    selected_flows+=("$flow")
  fi
done

[ "${#selected_flows[@]}" -gt 0 ] || {
  echo "unknown flow: $FLOW_FILTER" >&2
  exit 64
}

for flow in "${selected_flows[@]}"; do
  script="benchmarks/open-notes/$flow.auto"
  [ -f "$script" ] || {
    echo "missing automation script: $script" >&2
    exit 1
  }
done

slug_value() {
  printf '%s' "$1" | tr -cs 'A-Za-z0-9._-' '-' | sed 's/^-//; s/-$//'
}

unique_path() {
  base="$1"
  candidate="$base"
  suffix=1
  while [ -e "$candidate" ]; do
    candidate="$base-$suffix"
    suffix=$((suffix + 1))
  done
  printf '%s\n' "$candidate"
}

GENERATED_AT_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ARTIFACT_STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
GIT_COMMIT="$(git rev-parse HEAD 2>/dev/null || printf 'unknown')"
GIT_SHORT="$(git rev-parse --short HEAD 2>/dev/null || printf 'unknown')"
GIT_DIRTY=false
if [ -n "$(git status --porcelain 2>/dev/null || true)" ]; then
  GIT_DIRTY=true
fi
APK_SHA256="$(shasum -a 256 "$OPENNOTES_APK" | awk '{print $1}')"
BUILD_MODE="release"
PREFIX_FRESH=true
FLOW_SLUG="$(slug_value "$FLOW_FILTER")"
[ -n "$FLOW_SLUG" ] || FLOW_SLUG=all

if [ -z "$ARTIFACT_DIR" ]; then
  ARTIFACT_DIR="$(unique_path "$ARTIFACT_ROOT/${ARTIFACT_STAMP}_${GIT_SHORT}_${MODE}_${FLOW_SLUG}_${RUNS}runs")"
fi
ARTIFACT_RAW_DIR="$ARTIFACT_DIR/raw"
mkdir -p "$ARTIFACT_DIR"

if [ -z "$WORK_DIR" ]; then
  WORK_DIR="$ARTIFACT_RAW_DIR"
  CLEAN_WORK_DIR=0
else
  CLEAN_WORK_DIR=0
fi
mkdir -p "$WORK_DIR"

if [ "${KEEP_BENCH_WORK_DIR:-0}" != 1 ] && [ "$CLEAN_WORK_DIR" = 1 ]; then
  trap 'rm -rf "$WORK_DIR"' EXIT
else
  echo "bench-open-notes: work dir $WORK_DIR"
fi

{
  printf 'cwd=%s\n' "$(pwd)"
  printf 'generated_at_utc=%s\n' "$GENERATED_AT_UTC"
  printf 'command=scripts/bench_open_notes_usage.sh'
  for arg in "${ORIGINAL_ARGS[@]}"; do
    printf ' %s' "$arg"
  done
  printf '\n'
} >"$ARTIFACT_DIR/command.txt"

echo "bench-open-notes: artifact dir $ARTIFACT_DIR"

echo "bench-open-notes: release build"
cargo build --release --offline >/dev/null

BIN="target/release/winedroid"

WINEDROID_ENV_RESET=(
  -u WINEDROID_TRACE
  -u WINEDROID_DEX_TRACE
  -u WINEDROID_METHOD_TRACE
  -u WINEDROID_PREFIX
  -u WINEDROID_HEADLESS
  -u WINEDROID_AUTOMATION_SCRIPT
  -u WINEDROID_AUTOMATION_LOG
  -u WINEDROID_AUTOMATION_OPCODE_LOG
  -u WINEDROID_AUTOMATION_USAGE_LOG
  -u WINEDROID_AUTOMATION_RENDER_LOG
  -u WINEDROID_AUTOMATION_METHOD_LOG
  -u WINEDROID_AUTOMATION_INTERACTIVE
  -u WINEDROID_WINDOW_AUTOCLOSE_MS
  -u WINEDROID_RENDER_DUMP_PNG
  -u WINEDROID_UI_DUMP
  -u WINEDROID_MISSING_PRIMITIVE_LOG
  -u WD_OPCODE_HISTOGRAM
  -u WD_OPCODE_STEP_HISTOGRAM
  -u WD_METHOD_TIME_HISTOGRAM
)

for flow in "${selected_flows[@]}"; do
  for run in $(seq 1 "$RUNS"); do
    prefix="$WORK_DIR/prefix-$flow-$run"
    csv="$WORK_DIR/$flow.$run.csv"
    usage_csv="$WORK_DIR/$flow.$run.usage.csv"
    out="$WORK_DIR/$flow.$run.out"
    rm -rf "$prefix"

    extra_env=()
    case "$MODE" in
      headline)
        ;;
      opcode)
        extra_env+=(WD_OPCODE_HISTOGRAM=1)
        extra_env+=(WD_OPCODE_STEP_HISTOGRAM=1)
        extra_env+=(WINEDROID_AUTOMATION_OPCODE_LOG="$WORK_DIR/$flow.$run.opcodes.csv")
        ;;
      trace)
        extra_env+=(WINEDROID_TRACE=perf,runtime,ui,render,jank)
        extra_env+=(WINEDROID_AUTOMATION_RENDER_LOG="$WORK_DIR/$flow.$run.render.csv")
        extra_env+=(WINEDROID_AUTOMATION_METHOD_LOG="$WORK_DIR/$flow.$run.method.csv")
        ;;
      *)
        echo "internal bad mode: $MODE" >&2
        exit 70
        ;;
    esac

    echo "bench-open-notes: $flow run $run/$RUNS ($MODE)"
    command_env=(
      WD_NO_JIT=1
      WINEDROID_HEADLESS=1
      WINEDROID_PREFIX="$prefix"
      WINEDROID_AUTOMATION_SCRIPT="benchmarks/open-notes/$flow.auto"
      WINEDROID_AUTOMATION_LOG="$csv"
      WINEDROID_AUTOMATION_USAGE_LOG="$usage_csv"
      WINEDROID_WINDOW_AUTOCLOSE_MS=200
    )
    if [ "${#extra_env[@]}" -gt 0 ]; then
      command_env+=("${extra_env[@]}")
    fi

    /usr/bin/time -lp env "${WINEDROID_ENV_RESET[@]}" \
      "${command_env[@]}" \
      "$BIN" "$OPENNOTES_APK" >"$out" 2>&1
  done
done

if [ "$WORK_DIR" != "$ARTIFACT_RAW_DIR" ]; then
  mkdir -p "$ARTIFACT_RAW_DIR"
  cp -R "$WORK_DIR"/. "$ARTIFACT_RAW_DIR"/
fi

python3 - "$WORK_DIR" "$RUNS" "$MODE" "$RESULTS" "$ARTIFACT_DIR" "$ARTIFACT_RAW_DIR" "$GENERATED_AT_UTC" "$GIT_COMMIT" "$GIT_DIRTY" "$OPENNOTES_APK" "$APK_SHA256" "$BUILD_MODE" "$PREFIX_FRESH" "${selected_flows[@]}" <<'PYEOF'
import csv
import json
import os
import statistics
import sys
from pathlib import Path

work_dir = Path(sys.argv[1])
runs = int(sys.argv[2])
mode = sys.argv[3]
results_path = Path(sys.argv[4])
artifact_dir = Path(sys.argv[5])
artifact_raw_dir = Path(sys.argv[6])
generated_at_utc = sys.argv[7]
git_commit = sys.argv[8]
git_dirty = sys.argv[9] == "true"
apk_path = sys.argv[10]
apk_sha256 = sys.argv[11]
build_mode = sys.argv[12]
prefix_fresh = sys.argv[13] == "true"
flows = sys.argv[14:]

script_env_names = [
    "WINEDROID_OPENNOTES_APK",
    "BENCH_RUNS",
    "BENCH_WORK_DIR",
    "BENCH_ARTIFACT_ROOT",
    "BENCH_ARTIFACT_DIR",
    "KEEP_BENCH_WORK_DIR",
]

runtime_env = {
    "WD_NO_JIT": "1",
    "WINEDROID_HEADLESS": "1",
    "WINEDROID_PREFIX": str(work_dir / "prefix-<flow>-<run>"),
    "WINEDROID_AUTOMATION_SCRIPT": "benchmarks/open-notes/<flow>.auto",
    "WINEDROID_AUTOMATION_LOG": str(work_dir / "<flow>.<run>.csv"),
    "WINEDROID_AUTOMATION_USAGE_LOG": str(work_dir / "<flow>.<run>.usage.csv"),
    "WINEDROID_WINDOW_AUTOCLOSE_MS": "200",
}

if mode == "opcode":
    runtime_env["WD_OPCODE_HISTOGRAM"] = "1"
    runtime_env["WD_OPCODE_STEP_HISTOGRAM"] = "1"
    runtime_env["WINEDROID_AUTOMATION_OPCODE_LOG"] = str(work_dir / "<flow>.<run>.opcodes.csv")
elif mode == "trace":
    runtime_env["WINEDROID_TRACE"] = "perf,runtime,ui,render,jank"
    runtime_env["WINEDROID_AUTOMATION_RENDER_LOG"] = str(work_dir / "<flow>.<run>.render.csv")
    runtime_env["WINEDROID_AUTOMATION_METHOD_LOG"] = str(work_dir / "<flow>.<run>.method.csv")

metadata = {
    "generated_at_utc": generated_at_utc,
    "artifact_dir": str(artifact_dir),
    "raw_dir": str(artifact_raw_dir),
    "work_dir": str(work_dir),
    "git": {
        "commit": git_commit,
        "dirty": git_dirty,
    },
    "build": {
        "mode": build_mode,
        "binary": "target/release/winedroid",
    },
    "apk": {
        "path": apk_path,
        "sha256": apk_sha256,
    },
    "prefix": {
        "fresh_per_run": prefix_fresh,
    },
    "env": {name: os.environ.get(name) for name in script_env_names},
    "runtime_env": runtime_env,
}

def median(values):
    if not values:
        return None
    return statistics.median(values)

def percentile(values, pct):
    if not values:
        return None
    ordered = sorted(values)
    index = int(round((len(ordered) - 1) * pct / 100))
    return ordered[index]

def parse_time_output(path):
    fields = {
        "real_s": None,
        "user_s": None,
        "sys_s": None,
        "instructions": None,
        "cycles": None,
    }
    with path.open() as handle:
        for line in handle:
            parts = line.strip().split()
            if len(parts) < 2:
                continue
            if parts[0] == "real":
                fields["real_s"] = float(parts[1])
            elif parts[0] == "user":
                fields["user_s"] = float(parts[1])
            elif parts[0] == "sys":
                fields["sys_s"] = float(parts[1])
            elif parts[-2:] == ["instructions", "retired"]:
                fields["instructions"] = int(parts[0])
            elif parts[-2:] == ["cycles", "elapsed"]:
                fields["cycles"] = int(parts[0])
    return fields

def read_csv(path):
    with path.open() as handle:
        rows = list(csv.DictReader(handle))
    for row in rows:
        row["step"] = int(row["step"])
        row["t_inject_ms"] = int(row["t_inject_ms"])
        row["latency_ms"] = int(row["latency_ms"])
    return rows

def read_opcode_csv(path):
    if not path.exists():
        return []
    with path.open() as handle:
        rows = list(csv.DictReader(handle))
    for row in rows:
        row["step"] = int(row["step"])
        row["t_inject_ms"] = int(row["t_inject_ms"])
        row["latency_ms"] = int(row["latency_ms"])
        row["opcode_total"] = int(row["opcode_total"])
    return rows

opcode_bucket_names = [
    "calls_results_returns",
    "constants_moves_arith",
    "fields_statics",
    "branches_checks_casts",
    "arrays",
    "allocations",
    "other",
]

def opcode_bucket(opcode):
    if opcode in {0x0a, 0x0b, 0x0c} or 0x0e <= opcode <= 0x11 or 0x6e <= opcode <= 0x78:
        return "calls_results_returns"
    if 0x52 <= opcode <= 0x6d:
        return "fields_statics"
    if opcode in {0x1f, 0x20} or 0x28 <= opcode <= 0x3d:
        return "branches_checks_casts"
    if opcode == 0x21 or 0x23 <= opcode <= 0x26 or 0x44 <= opcode <= 0x51:
        return "arrays"
    if opcode == 0x22:
        return "allocations"
    if 0x01 <= opcode <= 0x09 or 0x12 <= opcode <= 0x1c or 0x2d <= opcode <= 0x31 or 0x7b <= opcode <= 0xe2:
        return "constants_moves_arith"
    return "other"

def opcode_buckets(top_opcodes):
    buckets = {name: 0 for name in opcode_bucket_names}
    if not top_opcodes:
        return buckets
    for entry in top_opcodes.split(";"):
        if not entry:
            continue
        parts = entry.split(":")
        if len(parts) != 3:
            continue
        opcode = int(parts[0], 16)
        count = int(parts[2])
        buckets[opcode_bucket(opcode)] += count
    return buckets

def optional_int(value):
    if value is None or value == "":
        return None
    return int(value)

def read_usage_csv(path):
    if not path.exists():
        return []
    with path.open() as handle:
        rows = list(csv.DictReader(handle))
    for row in rows:
        row["step"] = int(row["step"])
        row["t_inject_ms"] = int(row["t_inject_ms"])
        row["latency_ms"] = int(row["latency_ms"])
        row["user_us"] = optional_int(row["user_us"])
        row["system_us"] = optional_int(row["system_us"])
        row["instructions"] = optional_int(row["instructions"])
        row["cycles"] = optional_int(row["cycles"])
    return rows

def read_render_csv(path):
    if not path.exists():
        return []
    with path.open() as handle:
        rows = list(csv.DictReader(handle))
    metric_names = [
        "frame_count",
        "work_frame_count",
        "dropped_frame_count",
        "jank_frame_count",
        "frame_total_us",
        "frame_max_us",
        "traversal_count",
        "traversal_us",
        "measure_layout_count",
        "measure_layout_us",
        "draw_count",
        "draw_us",
        "retained_replay_count",
        "retained_replay_us",
        "surface_read_count",
        "surface_read_us",
        "surface_present_count",
        "surface_present_us",
        "host_redraw_requests",
    ]
    for row in rows:
        row["step"] = int(row["step"])
        row["t_inject_ms"] = int(row["t_inject_ms"])
        row["latency_ms"] = int(row["latency_ms"])
        for metric in metric_names:
            row[metric] = int(row[metric])
    return rows

def read_method_csv(path):
    if not path.exists():
        return []
    with path.open() as handle:
        rows = list(csv.DictReader(handle))
    metric_names = [
        "call_count",
        "call_us",
        "field_op_count",
        "field_op_us",
        "array_op_count",
        "array_op_us",
        "branch_count",
        "branch_us",
        "object_alloc_count",
        "object_alloc_us",
        "builtin_call_count",
        "builtin_call_us",
        "render_callback_count",
        "render_callback_us",
    ]
    for row in rows:
        row["step"] = int(row["step"])
        row["t_inject_ms"] = int(row["t_inject_ms"])
        row["latency_ms"] = int(row["latency_ms"])
        for metric in metric_names:
            row[metric] = int(row[metric])
    return rows

def first_action(rows, prefix):
    for index, row in enumerate(rows):
        if row["action"].startswith(prefix):
            return index
    return None

def immediate_waitidle_completion(rows, start_index):
    if start_index is None:
        return None
    next_index = start_index + 1
    if next_index >= len(rows):
        return None
    row = rows[next_index]
    if not row["action"].startswith("waitidle"):
        return None
    start = rows[start_index]["t_inject_ms"]
    return row["t_inject_ms"] + row["latency_ms"] - start

def final_time(rows):
    if not rows:
        return None
    last = rows[-1]
    return last["t_inject_ms"] + last["latency_ms"]

def text_latencies(rows):
    return [row["latency_ms"] for row in rows if row["action"].startswith("text ")]

summary = {
    "metadata": metadata,
    "mode": mode,
    "runs": runs,
    "flows": {},
}

for flow in flows:
    flow_samples = {
        "real_ms": [],
        "user_ms": [],
        "sys_ms": [],
        "instructions": [],
        "cycles": [],
        "final_automation_ms": [],
        "editor_open_visible_ms": [],
        "focus_visible_ms": [],
        "keystroke_visible_ms": [],
        "typing_text_median_ms": [],
        "typing_text_p95_ms": [],
    }
    opcode_steps = {}
    usage_steps = {}
    render_steps = {}
    method_steps = {}

    for run in range(1, runs + 1):
        rows = read_csv(work_dir / f"{flow}.{run}.csv")
        opcode_rows = read_opcode_csv(work_dir / f"{flow}.{run}.opcodes.csv")
        usage_rows = read_usage_csv(work_dir / f"{flow}.{run}.usage.csv")
        render_rows = read_render_csv(work_dir / f"{flow}.{run}.render.csv")
        method_rows = read_method_csv(work_dir / f"{flow}.{run}.method.csv")
        times = parse_time_output(work_dir / f"{flow}.{run}.out")
        if times["real_s"] is not None:
            flow_samples["real_ms"].append(round(times["real_s"] * 1000))
        if times["user_s"] is not None:
            flow_samples["user_ms"].append(round(times["user_s"] * 1000))
        if times["sys_s"] is not None:
            flow_samples["sys_ms"].append(round(times["sys_s"] * 1000))
        if times["instructions"] is not None:
            flow_samples["instructions"].append(times["instructions"])
        if times["cycles"] is not None:
            flow_samples["cycles"].append(times["cycles"])
        flow_samples["final_automation_ms"].append(final_time(rows))

        editor_index = first_action(rows, "tap 339:716")
        editor_visible = immediate_waitidle_completion(rows, editor_index)
        if editor_visible is not None:
            flow_samples["editor_open_visible_ms"].append(editor_visible)

        focus_index = first_action(rows, "tap 40:115")
        focus_visible = immediate_waitidle_completion(rows, focus_index)
        if focus_visible is not None:
            flow_samples["focus_visible_ms"].append(focus_visible)

        key_index = first_action(rows, 'text "x"')
        key_visible = immediate_waitidle_completion(rows, key_index)
        if key_visible is not None:
            flow_samples["keystroke_visible_ms"].append(key_visible)

        typing = text_latencies(rows)
        if typing:
            flow_samples["typing_text_median_ms"].append(median(typing))
            flow_samples["typing_text_p95_ms"].append(percentile(typing, 95))

        for opcode_row in opcode_rows:
            key = f"{opcode_row['step']} {opcode_row['action']}"
            step = opcode_steps.setdefault(
                key,
                {
                    "opcode_total": [],
                    "top_opcodes": [],
                    "buckets": {name: [] for name in opcode_bucket_names},
                },
            )
            step["opcode_total"].append(opcode_row["opcode_total"])
            step["top_opcodes"].append(opcode_row["top_opcodes"])
            for bucket, count in opcode_buckets(opcode_row["top_opcodes"]).items():
                step["buckets"][bucket].append(count)

        for usage_row in usage_rows:
            key = f"{usage_row['step']} {usage_row['action']}"
            step = usage_steps.setdefault(
                key,
                {
                    "user_us": [],
                    "system_us": [],
                    "instructions": [],
                    "cycles": [],
                },
            )
            for metric in ["user_us", "system_us", "instructions", "cycles"]:
                if usage_row[metric] is not None:
                    step[metric].append(usage_row[metric])

        for render_row in render_rows:
            key = f"{render_row['step']} {render_row['action']}"
            step = render_steps.setdefault(key, {})
            for metric in [
                "frame_count",
                "work_frame_count",
                "dropped_frame_count",
                "jank_frame_count",
                "frame_total_us",
                "frame_max_us",
                "traversal_count",
                "traversal_us",
                "measure_layout_count",
                "measure_layout_us",
                "draw_count",
                "draw_us",
                "retained_replay_count",
                "retained_replay_us",
                "surface_read_count",
                "surface_read_us",
                "surface_present_count",
                "surface_present_us",
                "host_redraw_requests",
            ]:
                step.setdefault(metric, []).append(render_row[metric])

        for method_row in method_rows:
            key = f"{method_row['step']} {method_row['action']}"
            step = method_steps.setdefault(key, {})
            for metric in [
                "call_count",
                "call_us",
                "field_op_count",
                "field_op_us",
                "array_op_count",
                "array_op_us",
                "branch_count",
                "branch_us",
                "object_alloc_count",
                "object_alloc_us",
                "builtin_call_count",
                "builtin_call_us",
                "render_callback_count",
                "render_callback_us",
            ]:
                step.setdefault(metric, []).append(method_row[metric])

    summary["flows"][flow] = {}
    for metric, samples in flow_samples.items():
        if samples:
            summary["flows"][flow][metric] = {
                "median": median(samples),
                "samples": samples,
            }
    if opcode_steps:
        summary["flows"][flow]["opcode_steps"] = {}
        for key, data in opcode_steps.items():
            bucket_summary = {}
            for name, samples in data["buckets"].items():
                if samples:
                    bucket_summary[name + "_median"] = median(samples)
                    bucket_summary[name + "_samples"] = samples
            summary["flows"][flow]["opcode_steps"][key] = {
                "opcode_total_median": median(data["opcode_total"]),
                "opcode_total_samples": data["opcode_total"],
                "top_opcodes_samples": data["top_opcodes"],
                "buckets": bucket_summary,
            }
    if usage_steps:
        summary["flows"][flow]["usage_steps"] = {}
        for key, data in usage_steps.items():
            step_summary = {}
            for metric, samples in data.items():
                if samples:
                    step_summary[metric + "_median"] = median(samples)
                    step_summary[metric + "_samples"] = samples
            summary["flows"][flow]["usage_steps"][key] = step_summary
    if render_steps:
        summary["flows"][flow]["render_steps"] = {}
        for key, data in render_steps.items():
            step_summary = {}
            for metric, samples in data.items():
                if samples:
                    step_summary[metric + "_median"] = median(samples)
                    step_summary[metric + "_samples"] = samples
            summary["flows"][flow]["render_steps"][key] = step_summary
    if method_steps:
        summary["flows"][flow]["method_steps"] = {}
        for key, data in method_steps.items():
            step_summary = {}
            for metric, samples in data.items():
                if samples:
                    step_summary[metric + "_median"] = median(samples)
                    step_summary[metric + "_samples"] = samples
            summary["flows"][flow]["method_steps"][key] = step_summary

if "focused-control" in summary["flows"] and "keystroke" in summary["flows"]:
    control = summary["flows"]["focused-control"]
    key = summary["flows"]["keystroke"]
    paired = {}
    for metric in ["real_ms", "user_ms", "sys_ms", "instructions", "cycles"]:
        if metric in control and metric in key:
            paired[metric + "_delta"] = key[metric]["median"] - control[metric]["median"]
    summary["keystroke_minus_focused_control"] = paired

artifact_dir.mkdir(parents=True, exist_ok=True)
summary_json = json.dumps(summary, indent=2) + "\n"
artifact_summary_path = artifact_dir / "summary.json"
metadata_path = artifact_dir / "metadata.json"
metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")
artifact_summary_path.write_text(summary_json)
results_path.parent.mkdir(parents=True, exist_ok=True)
results_path.write_text(summary_json)

print(f"bench-open-notes: medians over {runs} run(s), mode={mode}")
for flow in flows:
    print(f"bench-open-notes: {flow}")
    metrics = summary["flows"][flow]
    for metric in [
        "real_ms",
        "user_ms",
        "instructions",
        "editor_open_visible_ms",
        "focus_visible_ms",
        "keystroke_visible_ms",
        "typing_text_median_ms",
        "typing_text_p95_ms",
        "final_automation_ms",
    ]:
        if metric in metrics:
            print(f"  {metric:28} {metrics[metric]['median']}")
    if "opcode_steps" in metrics:
        print("  opcode_steps")
        for key, data in metrics["opcode_steps"].items():
            fields = data.get("buckets", {}).get("fields_statics_median")
            if fields is None:
                print(f"    {key:32} {data['opcode_total_median']}")
            else:
                print(f"    {key:32} total={data['opcode_total_median']} fields_statics={fields}")
    if "usage_steps" in metrics:
        print(f"  usage_steps                  {len(metrics['usage_steps'])}")
    if "render_steps" in metrics:
        print(f"  render_steps                 {len(metrics['render_steps'])}")
    if "method_steps" in metrics:
        print(f"  method_steps                 {len(metrics['method_steps'])}")

if "keystroke_minus_focused_control" in summary:
    print("bench-open-notes: keystroke - focused-control")
    for metric, value in summary["keystroke_minus_focused_control"].items():
        print(f"  {metric:28} {value}")

print(f"bench-open-notes: wrote {results_path}")
print(f"bench-open-notes: artifact summary {artifact_summary_path}")
print(f"bench-open-notes: artifact metadata {metadata_path}")
PYEOF
