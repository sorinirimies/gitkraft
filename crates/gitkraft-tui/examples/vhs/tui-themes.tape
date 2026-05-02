# VHS tape — GitKraft TUI · Theme Selector
# Showcases cycling through the 28 built-in colour themes.
#
# Run: vhs crates/gitkraft-tui/examples/vhs/tui-themes.tape

Output crates/gitkraft-tui/examples/vhs/generated/tui-themes.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.5

# ── Launch on the current repo (hidden from recording) ───────────────────────
Hide
Type "cargo run -q -p gitkraft-tui -- ."
Enter
Sleep 4s
Show

# ── Open theme picker ─────────────────────────────────────────────────────────
Type "T"
Sleep 1s

# ── Cycle through themes slowly so each one is visible ────────────────────────
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms

# ── Scroll back up to the top ─────────────────────────────────────────────────
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms

# ── Close theme picker ────────────────────────────────────────────────────────
Escape
Sleep 1s

# ── Final pause ───────────────────────────────────────────────────────────────
Sleep 2s

# ── Quit ──────────────────────────────────────────────────────────────────────
Type "q"
Sleep 500ms
