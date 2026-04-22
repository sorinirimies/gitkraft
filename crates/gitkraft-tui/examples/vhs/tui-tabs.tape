# VHS tape — GitKraft TUI · Multi-Repo Tab Management
# Showcases opening multiple repos in tabs, switching between them,
# and closing tabs.
#
# Run: vhs crates/gitkraft-tui/examples/vhs/tui-tabs.tape

Output crates/gitkraft-tui/examples/vhs/generated/tui-tabs.gif

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

# ── We are now in the main view with one tab ──────────────────────────────────
Sleep 1s

# ── Browse the commit log briefly ─────────────────────────────────────────────
Right
Sleep 500ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Enter
Sleep 1s

# ── Go back to branches pane ──────────────────────────────────────────────────
Left
Sleep 500ms

# ── Open a new tab ────────────────────────────────────────────────────────────
Type "N"
Sleep 1s

# ── Open directory browser to pick a repo ─────────────────────────────────────
Type "o"
Sleep 1s

# ── Navigate to find a repo ───────────────────────────────────────────────────
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms

# ── Enter directory ───────────────────────────────────────────────────────────
Enter
Sleep 800ms

# ── Navigate more ─────────────────────────────────────────────────────────────
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms

# ── Try to open as repo ───────────────────────────────────────────────────────
Enter
Sleep 2s

# ── Cancel and go back if not a repo ──────────────────────────────────────────
Escape
Sleep 800ms

# ── Switch between tabs ───────────────────────────────────────────────────────
Type "]"
Sleep 1s

Type "["
Sleep 1s

Type "]"
Sleep 1s

Type "["
Sleep 1s

# ── Close the current tab ─────────────────────────────────────────────────────
Type "W"
Sleep 1s

# ── Final pause — back to single tab ─────────────────────────────────────────
Sleep 2s

# ── Quit ──────────────────────────────────────────────────────────────────────
Type "q"
Sleep 500ms
