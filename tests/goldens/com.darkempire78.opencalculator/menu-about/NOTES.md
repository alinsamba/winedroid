# Open Calculator — menu-about journey goldens

- Journey: tests/journeys/com.darkempire78.opencalculator/menu-about.auto
  (open Support Us, close it, REOPEN the overflow menu, open About, confirm it
  opened, close it back to the keypad). Checkpoints: menu (overflow reopened),
  about (full-screen About), closed (back on the calculator).
- `<name>.xml`: winedroid's WINEDROID_UI_DUMP output, frozen as the regression
  golden and exact-diffed by validate.sh. Regenerate deliberately after an
  intended projection change.
- No `<name>.reference.xml` captured yet — capture on the emulator with
  scripts/capture_oracle.sh when doing parity work on this flow.
- This journey is the regression guard for the bug it was written to find:
  the overflow menu would not reopen after any dismissal, because the cached
  virtual-dispatch fast path in invoke_from_frame ran the app's obfuscated
  MenuInflater.inflate bytecode directly and skipped record_menu_inflate, so
  the second menu's items never entered runtime.menus. Fixed by guarding both
  cached fast paths (menu-inflate / setContentView route through self.invoke()).
- About is a full-screen view (not a dialog): closed via its
  about_back_button ImageView. `key back` is a separate unimplemented
  primitive.
