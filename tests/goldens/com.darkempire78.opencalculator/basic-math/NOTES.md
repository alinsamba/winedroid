# Open Calculator — basic-math journey goldens

- Journey: tests/journeys/com.darkempire78.opencalculator/basic-math.auto
  (launch, checkpoint `home`, tap 7 + 8 =, assert 15, checkpoint `result`).
- `home.xml` / `result.xml`: winedroid's own WINEDROID_UI_DUMP output, frozen
  as the regression golden. validate.sh replays the journey headless and
  requires an exact `diff` match. Regenerate deliberately (rerun the journey,
  review the diff, copy the new dumps here) whenever a projection change is
  intended.
- `home.reference.xml` / `result.reference.xml`: the same checkpoints captured
  on real Android for parity work (never machine-compared by the gate).
  Device: Android Emulator "Medium_Phone_API_36.0", Android 16 (API 36),
  arm64-v8a, google_apis_playstore image, 1080x2400. Captured 2026-07-17 with
  scripts/capture_oracle.sh.
- Known divergence at capture time (scripts/diff_ui_dump.py): `input` projects
  as a 20px right-edge sliver (drift 0.9), `sliding_layout_button` position,
  `resultDisplay` classed View instead of TextView, missing
  `history_sliding_layout_button` / clickable `constraintLayout3`, extra
  hidden `history_recylcle_view`. Fixing any of these should change the
  golden deliberately.
