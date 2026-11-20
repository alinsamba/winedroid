#!/usr/bin/env python3

import subprocess
import sys
import tempfile
from pathlib import Path

DIFFER = Path(__file__).with_name("diff_ui_dump.py")


def hierarchy(nodes):
    body = "".join(nodes)
    return (
        "<?xml version='1.0' encoding='UTF-8' standalone='yes' ?>\n"
        "<hierarchy rotation=\"0\">\n"
        "<node index=\"0\" text=\"\" resource-id=\"\" class=\"android.widget.FrameLayout\" "
        "bounds=\"[0,0][1000,2000]\">\n" + body + "</node>\n</hierarchy>\n"
    )


def node(rid="", text="", cls="android.widget.Button", bounds="[0,0][100,100]",
         clickable="true", checkable="false", checked="false", enabled="true"):
    return (
        f"<node index=\"0\" text=\"{text}\" resource-id=\"{rid}\" class=\"{cls}\" "
        f"checkable=\"{checkable}\" checked=\"{checked}\" clickable=\"{clickable}\" "
        f"enabled=\"{enabled}\" bounds=\"{bounds}\"/>\n"
    )


def run(expected_nodes, actual_nodes, *flags):
    with tempfile.TemporaryDirectory() as tmp:
        expected = Path(tmp) / "expected.xml"
        actual = Path(tmp) / "actual.xml"
        expected.write_text(hierarchy(expected_nodes))
        actual.write_text(hierarchy(actual_nodes))
        result = subprocess.run(
            [sys.executable, str(DIFFER), str(expected), str(actual), *flags],
            capture_output=True, text=True,
        )
        return result.returncode, result.stdout


CASES = []


def case(name):
    def register(fn):
        CASES.append((name, fn))
        return fn
    return register


@case("identical dumps pass")
def _(assert_):
    n = [node(rid="pkg:id/seven", text="7")]
    code, out = run(n, n)
    assert_(code == 0, f"expected pass, got {code}\n{out}")


@case("a missing node fails")
def _(assert_):
    code, out = run([node(rid="pkg:id/seven", text="7")], [])
    assert_(code == 1 and "MISSING" in out, out)


@case("an extra node fails")
def _(assert_):
    code, out = run([], [node(rid="pkg:id/seven", text="7")])
    assert_(code == 1 and "EXTRA" in out, out)


@case("a text difference on a joined node fails")
def _(assert_):
    code, out = run(
        [node(rid="pkg:id/display", text="15", cls="android.widget.TextView", clickable="false")],
        [node(rid="pkg:id/display", text="16", cls="android.widget.TextView", clickable="false")],
    )
    assert_(code == 1 and "text expected '15'" in out, out)


@case("a clickable-state difference fails")
def _(assert_):
    code, out = run(
        [node(rid="pkg:id/seven", text="7", clickable="false")],
        [node(rid="pkg:id/seven", text="7", clickable="true")],
    )
    assert_(code == 1 and "clickable" in out, out)


@case("button vs imagebutton is the same role and passes")
def _(assert_):
    code, out = run(
        [node(rid="pkg:id/point", cls="android.widget.Button")],
        [node(rid="pkg:id/point", cls="android.widget.ImageButton")],
    )
    assert_(code == 0, out)


@case("small bounds drift within tolerance is silent and passes")
def _(assert_):
    code, out = run(
        [node(rid="pkg:id/seven", text="7", bounds="[0,0][100,100]")],
        [node(rid="pkg:id/seven", text="7", bounds="[20,0][120,100]")],
    )
    assert_(code == 0 and "GEOMETRY" not in out, out)


@case("large bounds drift over tolerance notes but stays non-fatal")
def _(assert_):
    code, out = run(
        [node(rid="pkg:id/seven", text="7", bounds="[0,0][100,100]")],
        [node(rid="pkg:id/seven", text="7", bounds="[200,0][300,100]")],
    )
    assert_(code == 0 and "GEOMETRY" in out, out)


@case("id-less nodes join by text")
def _(assert_):
    n = [node(text="Close", cls="android.widget.Button", clickable="false")]
    code, out = run(n, n)
    assert_(code == 0, out)


@case("structural containers are filtered from both sides")
def _(assert_):
    container = (
        "<node index=\"0\" text=\"\" resource-id=\"pkg:id/wrapper\" "
        "class=\"android.widget.LinearLayout\" clickable=\"false\" checkable=\"false\" "
        "checked=\"false\" enabled=\"true\" bounds=\"[0,0][100,100]\">"
        + node(rid="pkg:id/seven", text="7")
        + "</node>\n"
    )
    code, out = run([container], [node(rid="pkg:id/seven", text="7")])
    assert_(code == 0, f"container should be ignored, got:\n{out}")


def main():
    failures = 0
    for name, fn in CASES:
        errors = []
        fn(lambda ok, detail="": errors.append(detail) if not ok else None)
        if errors:
            failures += 1
            print(f"FAIL {name}: {errors[0]}")
        else:
            print(f"ok   {name}")
    print(f"\n{len(CASES) - failures}/{len(CASES)} passed")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
