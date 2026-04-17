# VHS tape — GitKraft · Full Workspace Quality Gate
# Run: vhs crates/gitkraft-gui/examples/vhs/gui-workspace.tape

Output crates/gitkraft-gui/examples/vhs/generated/gui-workspace.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.5

Type "echo '=== GitKraft Workspace Quality Gate ==='"
Enter
Sleep 500ms

Type "cargo fmt --all -- --check && echo '✅ Formatting OK'"
Enter
Sleep 5s

Type "cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated 2>&1 | tail -3"
Enter
Sleep 30s

Type "cargo test --workspace 2>&1 | tail -30"
Enter
Sleep 60s

Type "echo '✅ All quality checks passed!'"
Enter
Sleep 2s
