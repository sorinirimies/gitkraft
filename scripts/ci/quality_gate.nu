#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — CI Quality Gate
# ──────────────────────────────────────────────────────────────────────────────
# Runs the full quality-gate sequence used by both CI and release workflows:
#   1. cargo fmt --check
#   2. cargo clippy (deny warnings, allow deprecated)
#   3. cargo test
#
# Usage:
#   nu scripts/ci/quality_gate.nu              # run all three checks
#   nu scripts/ci/quality_gate.nu --skip-fmt   # skip formatting check
#   nu scripts/ci/quality_gate.nu --skip-test  # skip test suite
#
# Exit codes:
#   0  — all checks passed
#   1  — one or more checks failed
# ──────────────────────────────────────────────────────────────────────────────

def green [msg: string] { $"(ansi green)($msg)(ansi reset)" }
def red   [msg: string] { $"(ansi red)($msg)(ansi reset)" }
def cyan  [msg: string] { $"(ansi cyan)($msg)(ansi reset)" }

def step [label: string] {
    print $"(cyan '▶') ($label)"
}

def main [
    --skip-fmt   # Skip the cargo fmt check
    --skip-test  # Skip the cargo test suite
] {
    print ""
    print (cyan "══════════════════════════════════════════════════════════")
    print (cyan "  GitKraft — Quality Gate")
    print (cyan "══════════════════════════════════════════════════════════")
    print ""

    mut failed = false

    # ── 1. Formatting ─────────────────────────────────────────────────────────
    if not $skip_fmt {
        step "cargo fmt --all -- --check"
        let result = (do { cargo fmt --all -- --check } | complete)
        if $result.exit_code != 0 {
            print (red "  ✗ Formatting check failed.")
            if ($result.stderr | str trim | is-not-empty) {
                print $result.stderr
            }
            if ($result.stdout | str trim | is-not-empty) {
                print $result.stdout
            }
            $failed = true
        } else {
            print (green "  ✔ Formatting OK")
        }
        print ""
    } else {
        print "  ⏭ Skipping cargo fmt"
        print ""
    }

    # ── 2. Clippy ─────────────────────────────────────────────────────────────
    step "cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated"
    let clippy = (do {
        cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated
    } | complete)
    if $clippy.exit_code != 0 {
        print (red "  ✗ Clippy found warnings or errors.")
        if ($clippy.stderr | str trim | is-not-empty) {
            print $clippy.stderr
        }
        $failed = true
    } else {
        print (green "  ✔ Clippy passed")
    }
    print ""

    # ── 3. Tests ──────────────────────────────────────────────────────────────
    if not $skip_test {
        step "cargo test --workspace --all-features --all-targets"
        let test_result = (do {
            cargo test --workspace --all-features --all-targets
        } | complete)
        if $test_result.exit_code != 0 {
            print (red "  ✗ Tests failed.")
            if ($test_result.stderr | str trim | is-not-empty) {
                print $test_result.stderr
            }
            if ($test_result.stdout | str trim | is-not-empty) {
                print $test_result.stdout
            }
            $failed = true
        } else {
            print (green "  ✔ All tests passed")
        }
        print ""
    } else {
        print "  ⏭ Skipping cargo test"
        print ""
    }

    # ── Summary ───────────────────────────────────────────────────────────────
    if $failed {
        print (red "══════════════════════════════════════════════════════════")
        print (red "  ✗ Quality gate FAILED")
        print (red "══════════════════════════════════════════════════════════")
        exit 1
    }

    print (green "══════════════════════════════════════════════════════════")
    print (green "  ✔ Quality gate passed")
    print (green "══════════════════════════════════════════════════════════")
}
