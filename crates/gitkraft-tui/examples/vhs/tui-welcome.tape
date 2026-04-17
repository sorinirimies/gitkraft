# VHS tape — GitKraft TUI · Welcome Screen & Directory Browser
# Shows the welcome screen with ASCII art, recent repos, and the
# interactive directory browser.
#
# Run: vhs crates/gitkraft-tui/examples/vhs/tui-welcome.tape

Output crates/gitkraft-tui/examples/vhs/generated/tui-welcome.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.5

# ── Launch without a repo (welcome screen) ───────────────────────────────────
Type "cargo run -p gitkraft-tui"
Enter
Sleep 4s

# ── Admire the welcome screen ────────────────────────────────────────────────
Sleep 2s

# ── Open directory browser ───────────────────────────────────────────────────
Type "o"
Sleep 1s

# ── Navigate directories ─────────────────────────────────────────────────────
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms

# ── Enter a directory ─────────────────────────────────────────────────────────
Enter
Sleep 800ms

# ── Navigate more ─────────────────────────────────────────────────────────────
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms

# ── Go back up ────────────────────────────────────────────────────────────────
Backspace
Sleep 800ms

# ── Go back up again ──────────────────────────────────────────────────────────
Backspace
Sleep 800ms

# ── Cancel and return to welcome ──────────────────────────────────────────────
Escape
Sleep 1s

# ── Final pause ───────────────────────────────────────────────────────────────
Sleep 2s

# ── Quit ──────────────────────────────────────────────────────────────────────
Type "q"
Sleep 500ms
