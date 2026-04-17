# VHS tape — GitKraft TUI · Build & Test Verification
# Run: vhs crates/gitkraft-tui/examples/vhs/tui-build.tape

Output crates/gitkraft-tui/examples/vhs/generated/tui-build.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.5

Type "rustc --version && cargo --version"
Enter
Sleep 1s

Type "ls -la crates/"
Enter
Sleep 1s

Type "cargo check -p gitkraft-tui -q 2>&1 | tail -5"
Enter
Sleep 20s

Type "cargo test -p gitkraft-tui --lib 2>&1 | tail -20"
Enter
Sleep 30s

Type "cargo clippy -p gitkraft-tui -- -D warnings 2>&1 | tail -5"
Enter
Sleep 20s

Type "cargo build -p gitkraft-tui --release -q && echo 'Build OK'"
Enter
Sleep 60s

Type "ls -lh target/release/gitkraft-tui"
Enter
Sleep 1s

Sleep 2s
