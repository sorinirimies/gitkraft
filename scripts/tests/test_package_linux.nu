#!/usr/bin/env nu
# ── GitKraft · test_package_linux.nu ────────────────────────────────────────
# Tests for scripts/ci/package_linux.nu — Linux .deb/.rpm packaging helpers.
#
# These are regression tests for a CI failure where the two `rpmbuild`
# `run-external` calls were split across multiple lines with no valid Nu
# line-continuation, producing a `nu::parser::parse_mismatch` ("expected
# operator") instead of a normal rpmbuild invocation — plus:
# - a second, latent parse error from an unsupported `\.` escape inside a
#   double-quoted string used to filter package files,
# - a third, purely *runtime* bug (see gui-deb-control tests below) that
#   only surfaced once the first two were fixed and CI actually reached
#   that code path,
# - a fourth bug where cross-architecture rpm builds (e.g. building an
#   aarch64 .rpm on an x86_64 CI runner) failed with "No compatible
#   architectures found for build", which was (wrongly) "fixed" by adding
#   `--target <arch>-linux`,
# - a fifth bug: passing --target made things *worse* for genuine
#   cross-arch builds, since it tells rpmbuild to validate cross-build
#   feasibility against the host's compatible-architecture table (which
#   Ubuntu's rpm package doesn't have aarch64<->x86_64 entries for) --
#   producing the exact same error, and
# - a sixth bug: dropping --target again *still* didn't help (CI run
#   30752224652), because rpmbuild validates `BuildArch:` in the .spec
#   against that same host-compatibility table regardless of --target.
#   aarch64 and x86_64 are simply never compatible in that table on this
#   rpm build, with no flag to override it for a genuine foreign
#   architecture.
#
# The actual fix replaces rpmbuild + hand-written .spec files entirely
# with `alien`, which converts the already-built, already-correctly-tagged
# .deb directly into a .rpm without any host/target architecture
# compilation-feasibility check (it's pure repackaging of existing files,
# which is all this step ever needed to do).

use std/assert
use runner.nu *
use ../ci/package_linux.nu [alien-args, is-linux-package-file, gui-deb-control]

# ── alien-args ───────────────────────────────────────────────────────────────

def "test alien-args: returns exactly three args" [] {
    let args = (alien-args "dist/gitkraft-tui_1.1.6_amd64.deb")
    assert equal ($args | length) 3
}

def "test alien-args: is --to-rpm --scripts <deb_path>" [] {
    let args = (alien-args "dist/gitkraft-tui_1.1.6_amd64.deb")
    assert equal ($args | get 0) "--to-rpm"
    assert equal ($args | get 1) "--scripts"
    assert equal ($args | get 2) "dist/gitkraft-tui_1.1.6_amd64.deb"
}

def "test alien-args: does not pass --target or a BuildArch-style flag" [] {
    # Regression guard: unlike the old rpmbuild-based approach, alien takes
    # no architecture flag at all -- it reads the architecture directly from
    # the .deb being converted, so there's no host-compatibility gate to
    # accidentally trip.
    let args = (alien-args "dist/gitkraft_2.0.0_arm64.deb")
    assert (not ($args | any { |it| $it == "--target" }))
    assert (not ($args | any { |it| $it | str contains "BuildArch" }))
}

def "test alien-args: works for the GUI deb path too" [] {
    let deb = "dist/gitkraft_9.9.9_arm64.deb"
    let args = (alien-args $deb)
    assert equal ($args | length) 3
    assert equal ($args | get 2) $deb
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

# ── gui-deb-control ───────────────────────────────────────────────────────────
# Regression tests for a second, purely *runtime* bug that the parse fix
# above could not catch: the literal parenthesised text "(Elm Architecture)"
# inside a `$"..."` interpolated string was parsed by Nu as a nested command
# call (`Elm Architecture`) instead of literal text, since any unescaped
# `(...)` inside a Nu interpolated string is evaluated as an expression. This
# only surfaced once the earlier parse-mismatch bug was fixed and CI actually
# reached this code path -- so these tests exercise the generated text
# directly rather than relying on a full end-to-end packaging run.

def "test gui-deb-control: contains the literal Elm Architecture parenthetical" [] {
    let control = (gui-deb-control "1.1.6" "amd64")
    assert ($control | str contains "(Elm Architecture)")
}

def "test gui-deb-control: interpolates version and architecture" [] {
    let control = (gui-deb-control "9.9.9" "arm64")
    assert ($control | str contains "Version: 9.9.9")
    assert ($control | str contains "Architecture: arm64")
}

def "test gui-deb-control: has the expected package name and maintainer" [] {
    let control = (gui-deb-control "1.0.0" "amd64")
    assert ($control | str contains "Package: gitkraft")
    assert ($control | str contains "Maintainer: Sorin Irimies")
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

def "test package_linux.nu: uses into glob for the dynamic .rpm lookup pattern" [] {
    # Regression guard: `ls $"(...)/*.rpm"` (a plain interpolated string) is
    # ambiguous across nu versions -- some treat it as a literal filename
    # lookup ("DoNotExpand") rather than a glob, which fails even when a
    # matching file exists. `into glob` forces correct glob expansion.
    let text = (open --raw ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
    assert ($text | str contains "into glob")
}

def "test package_linux.nu: no longer invokes rpmbuild" [] {
    # Regression guard: rpmbuild + hand-written .spec files were replaced
    # entirely by alien (see module header for the full saga of why). Prose
    # comments are allowed to mention "rpmbuild" (to explain the history);
    # the actual external-command invocation must be gone.
    let text = (open --raw ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
    assert (not ($text | str contains "run-external \"rpmbuild\""))
    assert (not ($text | str contains "%install"))
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
