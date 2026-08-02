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
# - a third, purely *runtime* bug (see gui-deb-control / gui-rpm-spec tests
#   below) that only surfaced once the first two were fixed and CI actually
#   reached that code path,
# - a fourth bug where cross-architecture rpm builds (e.g. building an
#   aarch64 .rpm on an x86_64 CI runner) failed with "No compatible
#   architectures found for build", which was (wrongly) "fixed" by adding
#   `--target <arch>-linux`, and
# - a fifth bug: passing --target actually made things *worse* for genuine
#   cross-arch builds — it tells rpmbuild to validate cross-build
#   feasibility against the host's compatible-architecture table, which
#   Ubuntu's rpm package doesn't have entries for (aarch64 on an x86_64
#   host), producing the exact same "No compatible architectures found for
#   build" error (CI run 30751058486). The actual fix is to drop --target
#   entirely: we aren't compiling anything, only packaging an already
#   cross-compiled, pre-built binary, and the package's architecture tag is
#   controlled purely by `BuildArch:` in the .spec file, which doesn't
#   trigger any host-compatibility validation.

use std/assert
use runner.nu *
use ../ci/package_linux.nu [rpmbuild-args, is-linux-package-file, gui-deb-control, gui-rpm-spec]

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

def "test rpmbuild-args: does not pass --target" [] {
    # Regression guard: --target was tried as a fix for cross-arch builds
    # but actually broke them further by triggering rpmbuild's
    # host-compatible-architecture validation, which Ubuntu's rpm package
    # doesn't have aarch64<->x86_64 entries for. The architecture is
    # controlled entirely by `BuildArch:` in the .spec file instead.
    let args = (rpmbuild-args "dist/rpmbuild" "unused.spec")
    assert (not ($args | any { |it| $it == "--target" }))
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

# ── gui-deb-control / gui-rpm-spec ───────────────────────────────────────────
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

def "test gui-rpm-spec: contains the literal Elm Architecture parenthetical" [] {
    let spec = (gui-rpm-spec "1.1.6" "x86_64" "Sun Aug 02 2026")
    assert ($spec | str contains "(Elm Architecture)")
}

def "test gui-rpm-spec: interpolates version, arch, and changelog date" [] {
    let spec = (gui-rpm-spec "2.3.4" "aarch64" "Mon Jan 01 2027")
    assert ($spec | str contains "Version:        2.3.4")
    assert ($spec | str contains "BuildArch:      aarch64")
    assert ($spec | str contains "* Mon Jan 01 2027")
}

def "test gui-rpm-spec: has the expected rpm sections" [] {
    let spec = (gui-rpm-spec "1.0.0" "x86_64" "Sun Aug 02 2026")
    assert ($spec | str contains "%description")
    assert ($spec | str contains "%install")
    assert ($spec | str contains "%files")
    assert ($spec | str contains "%changelog")
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
