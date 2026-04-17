# VHS tape — GitKraft GUI · Build & Test Verification
# Run: vhs crates/gitkraft-gui/examples/vhs/gui-build.tape

Output crates/gitkraft-gui/examples/vhs/generated/gui-build.gif

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

Type "cargo check -p gitkraft -q 2>&1 | tail -5"
Enter
Sleep 20s

Type "cargo test -p gitkraft --lib 2>&1 | tail -20"
Enter
Sleep 30s

Type "cargo clippy -p gitkraft -- -D warnings 2>&1 | tail -5"
Enter
Sleep 20s

Type "cargo build -p gitkraft --release -q && echo 'Build OK'"
Enter
Sleep 60s

Type "ls -lh target/release/gitkraft"
Enter
Sleep 1s

Type "cargo doc -p gitkraft --no-deps -q 2>&1 | tail -3 && echo 'Docs OK'"
Enter
Sleep 20s

Sleep 2s
