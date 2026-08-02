#!/usr/bin/env nu
# ── GitKraft · test_package_linux.nu ────────────────────────────────────────
# Tests for scripts/ci/package_linux.nu — Linux .deb/.rpm packaging helpers.
#
# These are regression tests for a CI failure where the two `rpmbuild`
# `run-external` calls were split across multiple lines with no valid Nu
# line-continuation, producing a `nu::parser::parse_mismatch` ("expected
# operator") instead of a normal rpmbuild invocation — plus a second, latent
# parse error from an unsupported `\.` escape inside a double-quoted string
# used to filter package files.

use std/assert
use runner.nu *
use ../ci/package_linux.nu [rpmbuild-args, is-linux-package-file]

# ── rpmbuild-args ────────────────────────────────────────────────────────────

def "test rpmbuild-args: returns exactly four args" [] {
    let args = (rpmbuild-args "dist/rpmbuild" "dist/rpmbuild/SPECS/gitkraft-tui.spec")
    assert equal ($args | length) 4
}

def "test rpmbuild-args: first two args are -bb and --define" [] {
    let args = (rpmbuild-args "dist/rpmbuild" "dist/rpmbuild/SPECS/gitkraft-tui.spec")
    assert equal ($args | get 0) "-bb"
    assert equal ($args | get 1) "--define"
}

def "test rpmbuild-args: --define value is a single arg starting with _topdir" [] {
    # Regression guard: previously "--define" and its value were emitted
    # correctly as two args already, but the whole call was broken across
    # lines with no continuation. Assert the --define *value* stays a single,
    # unsplit string containing both the "_topdir" key and the resolved path.
    let args = (rpmbuild-args "dist/rpmbuild" "dist/rpmbuild/SPECS/gitkraft-tui.spec")
    let define_value = ($args | get 2)
    assert ($define_value | str starts-with "_topdir ")
    assert ($define_value | str ends-with "dist/rpmbuild")
}

def "test rpmbuild-args: last arg is the spec path, unmodified" [] {
    let spec = "dist/rpmbuild/SPECS/gitkraft-tui.spec"
    let args = (rpmbuild-args "dist/rpmbuild" $spec)
    assert equal ($args | get 3) $spec
}

def "test rpmbuild-args: works for the GUI spec path too" [] {
    let spec = "dist/rpmbuild/SPECS/gitkraft.spec"
    let args = (rpmbuild-args "dist/rpmbuild" $spec)
    assert equal ($args | length) 4
    assert equal ($args | get 3) $spec
}

def "test rpmbuild-args: --define value embeds the given rpm_build directory" [] {
    let args = (rpmbuild-args "some/other/rpmbuild" "unused.spec")
    assert (($args | get 2) | str ends-with "some/other/rpmbuild")
}

# ── is-linux-package-file ────────────────────────────────────────────────────

def "test is-linux-package-file: matches .deb files" [] {
    assert (is-linux-package-file "gitkraft-tui_1.2.3_amd64.deb")
}

def "test is-linux-package-file: matches .rpm files" [] {
    assert (is-linux-package-file "gitkraft-1.2.3-1.x86_64.rpm")
}

def "test is-linux-package-file: rejects unrelated files" [] {
    assert (not (is-linux-package-file "gitkraft-tui"))
    assert (not (is-linux-package-file "gitkraft.tar.gz"))
    assert (not (is-linux-package-file "notes.txt"))
}

def "test is-linux-package-file: does not match substrings mid-name" [] {
    # ".deb"/".rpm" must be the actual file extension, not just present
    # somewhere earlier in the name.
    assert (not (is-linux-package-file "deb-tui-staging-dir"))
    assert (not (is-linux-package-file "rpmbuild-notes.md"))
}

def "test is-linux-package-file: empty string is not a package" [] {
    assert (not (is-linux-package-file ""))
}

# ── Parse-level regression guard ─────────────────────────────────────────────
# The original bug was a syntax error, not a logic error, so the most direct
# regression test is simply confirming the script still parses cleanly.

def "test package_linux.nu: parses without syntax errors" [] {
    assert (nu-check ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
}

def "test package_linux.nu: parses cleanly as a module too" [] {
    # Exercises the same `export def` parsing path used by `use ... [fn, ...]`
    # above, guarding against a regression that only manifests when the file
    # is imported as a module rather than run as a script.
    assert (nu-check --as-module ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
