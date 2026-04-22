# VHS tape — GitKraft TUI · Full Demo
# Showcases the terminal UI: welcome screen, repo browsing, commit log,
# file list, diff viewer, staging, and theme switching.
#
# Run: vhs crates/gitkraft-tui/examples/vhs/tui-demo.tape

Output crates/gitkraft-tui/examples/vhs/generated/tui-demo.gif

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


# ── Navigate branches ────────────────────────────────────────────────────────
# We start in the Branches pane
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 500ms

# ── Switch to Commit Log pane ─────────────────────────────────────────────────
Right
Sleep 500ms

# ── Scroll through commits ───────────────────────────────────────────────────
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms

# ── Select a commit → loads diff ──────────────────────────────────────────────
Enter
Sleep 2s

# ── Switch to Diff/Files pane ─────────────────────────────────────────────────
Right
Sleep 500ms

# ── Navigate files with arrow keys ────────────────────────────────────────────
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Up
Sleep 500ms
Up
Sleep 800ms

# ── Scroll the diff content ──────────────────────────────────────────────────
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 500ms

# ── Switch to Staging pane ────────────────────────────────────────────────────
Right
Sleep 500ms

# ── Back to Branches ──────────────────────────────────────────────────────────
Right
Sleep 800ms

# ── Open theme picker ────────────────────────────────────────────────────────
Type "T"
Sleep 1s

# Cycle through a few themes
Type "j"
Sleep 600ms
Type "j"
Sleep 600ms
Type "j"
Sleep 600ms
Type "j"
Sleep 600ms
Type "j"
Sleep 800ms

# Close theme picker
Escape
Sleep 1s

# ── Refresh ───────────────────────────────────────────────────────────────────
Type "r"
Sleep 2s

# ── Final pause ───────────────────────────────────────────────────────────────
Sleep 2s

# ── Quit ──────────────────────────────────────────────────────────────────────
Type "q"
Sleep 500ms
