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

# ── We are now in the main view with one tab (gitkraft) ───────────────────────
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

# ── Open directory browser (starts in ~/Projects — parent of current repo) ───
Type "o"
Sleep 1s

# ── Navigate to droidtui (a sibling repo in ~/Projects) ──────────────────────
# Browser starts at ~/Projects, entries sorted with dot-dirs first:
#   .tmux → act_runner → android-app-framework → Arch-Hyprland →
#   arrow-resilience-kit → bevy → cleansys → droidtui
# Cursor starts at index 0, so 7 presses of j to reach droidtui.
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms

# ── Select droidtui (it's a git repo, so it opens directly) ──────────────────
Enter
Sleep 3s

# ── We now have two tabs — explore the second repo ────────────────────────────
Sleep 1s

# ── Browse branches in the second repo ────────────────────────────────────────
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms

# ── Check commit log ──────────────────────────────────────────────────────────
Right
Sleep 500ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Enter
Sleep 1s

# ── Switch back to first tab (gitkraft) ───────────────────────────────────────
Type "["
Sleep 1s

# ── Switch to second tab (droidtui) ──────────────────────────────────────────
Type "]"
Sleep 1s

# ── Switch back again ─────────────────────────────────────────────────────────
Type "["
Sleep 1s

# ── Switch forward once more ──────────────────────────────────────────────────
Type "]"
Sleep 1s

# ── Close the second tab ──────────────────────────────────────────────────────
Type "W"
Sleep 1s

# ── Final pause — back to single tab (gitkraft) ──────────────────────────────
Sleep 2s

# ── Quit ──────────────────────────────────────────────────────────────────────
Type "q"
Sleep 500ms
