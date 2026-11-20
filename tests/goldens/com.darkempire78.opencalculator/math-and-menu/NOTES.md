# Open Calculator — math-and-menu journey goldens

- Journey: tests/journeys/com.darkempire78.opencalculator/math-and-menu.auto
  (1 + 2 x 3 / 4 = 2.5, then overflow menu, Support Us dialog, Close).
  Checkpoints: home, result, menu (overflow open), support (dialog),
  closed (back on the calculator).
- `<name>.xml`: winedroid's WINEDROID_UI_DUMP output, frozen as the regression
  golden. validate.sh replays the journey headless and requires an exact
  `diff` match. Regenerate deliberately after an intended projection change.
- `<name>.reference.xml`: the same checkpoints on real Android for parity work
  (never machine-compared by the gate).
  Device: Android Emulator "Medium_Phone_API_36.0", Android 16 (API 36),
  arm64-v8a, google_apis_playstore image, 1080x2400. Captured 2026-07-17 with
  scripts/capture_oracle.sh.
- Known divergence at capture time (scripts/diff_ui_dump.py), all real
  winedroid defects to fix, each of which should change a golden deliberately:
    - home/result/closed: `resultDisplay` classed View not TextView; missing
      `history_sliding_layout_button` and clickable `constraintLayout3`; extra
      hidden `history_recylcle_view`; keypad bounds drift ~0.05-0.07 and the
      `input` field projects far too narrow (see basic-math NOTES).
    - menu: overflow items mark the text node clickable=true where real
      Android marks the item's `title` TextView clickable=false (the row is the
      target). Defensible for semantic taps; noted, not yet reconciled.
    - support: the renderer draws the dialog title, but the UI dump omits the
      real Android `alertTitle` TextView node.
