#!/usr/bin/env python3

import argparse
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

CLI_DESCRIPTION = """Compare two uiautomator-shaped UI dumps from real Android and winedroid.

Both inputs use the same uiautomator XML schema. Structural containers and
system decor are filtered out, classes collapse to roles, and bounds are
compared as fractions of the root surface within --tolerance.

Nodes join by resource ID first and then by text for nodes without IDs. The
command exits nonzero for semantic differences. Geometry drift never fails the
comparison; drift beyond the tolerance is reported.

python3 scripts/diff_ui_dump.py emulator/home.xml winedroid/home.xml
"""

SYSTEM_IDS = {
    "android:id/content",
    "android:id/action_bar_root",
    "android:id/statusBarBackground",
    "android:id/navigationBarBackground",
}

ROLES = {
    "android.widget.Button": "button",
    "android.widget.ImageButton": "button",
    "android.widget.EditText": "edit",
    "android.widget.TextView": "label",
}


def parse_bounds(raw):
    match = re.fullmatch(r"\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]", raw or "")
    if not match:
        return None
    return tuple(int(group) for group in match.groups())


def content_frame(tree, root_bounds):
    left, top, right, bottom = root_bounds
    for node in tree.iter("node"):
        rid = node.get("resource-id", "")
        bounds = parse_bounds(node.get("bounds"))
        if not bounds:
            continue
        if rid == "android:id/statusBarBackground":
            top = max(top, bounds[3])
        if rid == "android:id/navigationBarBackground":
            bottom = min(bottom, bounds[1])
    return left, top, right, bottom


def normalize(path):
    tree = ET.parse(path)
    root_node = tree.getroot().find("node")
    if root_node is None:
        raise SystemExit(f"{path}: no root <node> in hierarchy")
    root_bounds = parse_bounds(root_node.get("bounds"))
    if not root_bounds or root_bounds[2] <= root_bounds[0] or root_bounds[3] <= root_bounds[1]:
        raise SystemExit(f"{path}: root node has unusable bounds")
    left, top, right, bottom = content_frame(tree, root_bounds)
    width = right - left
    height = bottom - top

    nodes = []
    container_ids = set()
    for node in tree.iter("node"):
        rid = node.get("resource-id", "")
        text = node.get("text", "")
        short_rid = rid.split(":id/")[-1] if ":id/" in rid else rid
        if rid in SYSTEM_IDS:
            continue
        is_container = (
            len(node.findall("node")) > 0
            and not text
            and node.get("clickable") != "true"
            and node.get("checkable") != "true"
        )
        if is_container:
            if short_rid:
                container_ids.add(short_rid)
            continue
        if not rid and not text:
            continue
        bounds = parse_bounds(node.get("bounds"))
        fractions = None
        if bounds:
            fractions = (
                (bounds[0] - left) / width,
                (bounds[1] - top) / height,
                (bounds[2] - left) / width,
                (bounds[3] - top) / height,
            )
        nodes.append(
            {
                "rid": rid,
                "short_rid": short_rid,
                "text": text,
                "role": ROLES.get(node.get("class", ""), "view"),
                "clickable": node.get("clickable") == "true",
                "checkable": node.get("checkable") == "true",
                "checked": node.get("checked") == "true",
                "enabled": node.get("enabled") == "true",
                "fractions": fractions,
            }
        )
    return nodes, container_ids


def join(expected, actual):
    def index(nodes, key):
        seen = {}
        table = {}
        for node in nodes:
            value = key(node)
            if not value:
                continue
            occurrence = seen.get(value, 0)
            seen[value] = occurrence + 1
            table[(value, occurrence)] = node
        return table

    expected_by_id = index(expected, lambda n: n["short_rid"])
    actual_by_id = index(actual, lambda n: n["short_rid"])
    pairs = []
    matched_actual = set()

    for key, node in expected_by_id.items():
        if key in actual_by_id:
            pairs.append((node, actual_by_id[key]))
            matched_actual.add(id(actual_by_id[key]))

    expected_unmatched = [n for n in expected if not any(n is p[0] for p in pairs)]
    actual_unmatched = [n for n in actual if id(n) not in matched_actual]

    expected_by_text = index(expected_unmatched, lambda n: n["text"])
    actual_by_text = index(actual_unmatched, lambda n: n["text"])
    for key, node in expected_by_text.items():
        if key in actual_by_text:
            pairs.append((node, actual_by_text[key]))
            matched_actual.add(id(actual_by_text[key]))

    expected_unmatched = [n for n in expected if not any(n is p[0] for p in pairs)]
    actual_unmatched = [n for n in actual if id(n) not in matched_actual]
    return pairs, expected_unmatched, actual_unmatched


def describe(node):
    if node["short_rid"]:
        label = node["short_rid"]
        if node["text"]:
            label += f" text={node['text']!r}"
    else:
        label = f"text={node['text']!r}"
    return f"{label} ({node['role']})"


def main():
    parser = argparse.ArgumentParser(
        description=CLI_DESCRIPTION,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("expected", type=Path, help="reference dump (real Android)")
    parser.add_argument("actual", type=Path, help="dump under test (winedroid)")
    parser.add_argument("--tolerance", type=float, default=0.05,
                        help="max bounds drift as a fraction of the screen (default 0.05)")
    parser.add_argument("--ignore-geometry", action="store_true")
    options = parser.parse_args()

    expected, expected_containers = normalize(options.expected)
    actual, actual_containers = normalize(options.actual)
    containers = expected_containers | actual_containers
    expected = [n for n in expected if n["short_rid"] not in containers]
    actual = [n for n in actual if n["short_rid"] not in containers]
    pairs, missing, extra = join(expected, actual)

    diffs = 0
    geometry_notes = []

    for node in missing:
        print(f"MISSING  {describe(node)} — on {options.expected.name} only")
        diffs += 1
    for node in extra:
        print(f"EXTRA    {describe(node)} — on {options.actual.name} only")
        diffs += 1

    for left, right in sorted(pairs, key=lambda p: p[0]["short_rid"] or p[0]["text"]):
        for field in ("text", "role", "clickable", "checkable", "checked", "enabled"):
            if left[field] != right[field]:
                print(
                    f"DIFF     {describe(left)}: {field} "
                    f"expected {left[field]!r}, got {right[field]!r}"
                )
                diffs += 1
        if options.ignore_geometry or not left["fractions"] or not right["fractions"]:
            continue
        drift = max(abs(a - b) for a, b in zip(left["fractions"], right["fractions"]))
        if drift > options.tolerance:
            geometry_notes.append(
                f"GEOMETRY {describe(left)}: bounds drift {drift:.3f} "
                f"(> {options.tolerance})"
            )

    for note in geometry_notes:
        print(note)

    print(
        f"compared {len(pairs)} matched nodes: {diffs} semantic diff(s), "
        f"{len(geometry_notes)} geometry note(s), "
        f"{len(missing)} missing, {len(extra)} extra"
    )
    sys.exit(1 if diffs else 0)


if __name__ == "__main__":
    main()
