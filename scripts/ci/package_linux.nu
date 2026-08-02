#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Linux packaging: .deb, .rpm, AppImage
# ──────────────────────────────────────────────────────────────────────────────
# Usage:
#   nu scripts/ci/package_linux.nu <version> <target>
#
# Requires: dpkg-deb (for .deb), rpmbuild (for .rpm), appimagetool (for AppImage)
# Tools are installed in the CI job that calls this script.
# ──────────────────────────────────────────────────────────────────────────────

# Build the argument list for the `rpmbuild` invocation used to build one
# .rpm package. Extracted into its own function so the exact shape of the
# args (how many, in what order, whether `--define` and its value are two
# separate args or accidentally merged/split) can be asserted directly in
# tests, rather than only being caught at CI time via a parser error.
#
# Regression guard: these args were previously spread across three lines
# with no valid Nu line-continuation between them, which produced a
# `nu::parser::parse_mismatch` ("expected operator") failure in CI instead
# of a normal rpmbuild invocation.
export def rpmbuild-args [
    rpm_build: string   # e.g. dist/rpmbuild
    spec_path: string   # e.g. dist/rpmbuild/SPECS/gitkraft-tui.spec
]: nothing -> list<string> {
    ["-bb" "--define" $"_topdir (pwd)/($rpm_build)" $spec_path]
}

# Whether a file name looks like a Linux package artifact (.deb or .rpm).
# Extracted so the glob/regex used by the final packaging summary can be
# unit tested directly instead of only being exercised as a side effect of
# `ls | where ...` at the end of a full packaging run.
export def is-linux-package-file [name: string]: nothing -> bool {
    ($name | str ends-with ".deb") or ($name | str ends-with ".rpm")
}

# Build the DEBIAN/control file contents for the desktop GUI .deb package.
#
# Extracted into its own pure function (no side effects, no external
# commands) so its exact text can be asserted in tests. This is a regression
# guard for a bug where the literal parenthesised text "(Elm Architecture)"
# inside a `$"..."` interpolated string was parsed by Nu as a *nested
# command call* (`Elm Architecture`) rather than literal text, since any
# unescaped `(...)` inside a Nu interpolated string is evaluated as an
# expression. That bug could not be caught by `nu-check` / parsing alone —
# it only failed at runtime, when the packaging step actually reached this
# code path.
export def gui-deb-control [version: string, arch: string]: nothing -> string {
    $"Package: gitkraft
Version: ($version)
Architecture: ($arch)
Maintainer: Sorin Irimies <sorinirimies@gmail.com>
Description: GitKraft — desktop GUI Git IDE written in Rust
 A mouse-driven desktop GUI for Git, built on Iced \(Elm Architecture\).
Homepage: https://github.com/sorinirimies/gitkraft
Depends: libxkbcommon0, libwayland-client0, libgl1
"
}

# Build the .spec file contents for the desktop GUI .rpm package. Same
# rationale as `gui-deb-control` above — kept pure and testable so the
# escaped parentheses stay literal text instead of silently regressing into
# a nested command call.
export def gui-rpm-spec [
    version: string
    rpm_arch: string
    rpm_date: string
]: nothing -> string {
    $"Name:           gitkraft
Version:        ($version)
Release:        1%{?dist}
Summary:        Desktop GUI Git IDE written in Rust
License:        MIT
URL:            https://github.com/sorinirimies/gitkraft
BuildArch:      ($rpm_arch)
Requires:       libxkbcommon, wayland-libs-client, mesa-libGL

%description
A mouse-driven desktop GUI for Git, built on Iced \(Elm Architecture\).

%install
mkdir -p %{buildroot}/usr/bin
install -m 755 %{_sourcedir}/gitkraft %{buildroot}/usr/bin/gitkraft

%files
/usr/bin/gitkraft

%changelog
* ($rpm_date) Sorin Irimies <sorinirimies@gmail.com> - ($version)-1
- Release ($version)
"
}

def main [
    version: string   # e.g. 0.7.7
    target: string    # e.g. x86_64-unknown-linux-gnu
] {
    let arch = if ($target | str contains "aarch64") { "arm64" } else { "amd64" }
    let rpm_arch = if ($target | str contains "aarch64") { "aarch64" } else { "x86_64" }
    let dist_dir = "dist"

    mkdir $dist_dir

    # ── .deb for gitkraft-tui ────────────────────────────────────────────────
    let tui_deb_root = $"($dist_dir)/deb-tui"
    mkdir $"($tui_deb_root)/DEBIAN"
    mkdir $"($tui_deb_root)/usr/bin"
    mkdir $"($tui_deb_root)/usr/share/doc/gitkraft-tui"

    cp $"target/($target)/release/gitkraft-tui" $"($tui_deb_root)/usr/bin/gitkraft-tui"

    $"Package: gitkraft-tui
Version: ($version)
Architecture: ($arch)
Maintainer: Sorin Irimies <sorinirimies@gmail.com>
Description: GitKraft TUI — terminal Git IDE written in Rust
 A keyboard-driven terminal UI for Git, built on Ratatui.
Homepage: https://github.com/sorinirimies/gitkraft
" | save -f $"($tui_deb_root)/DEBIAN/control"

    $"GitKraft TUI ($version)
Copyright 2024 Sorin Irimies
MIT License — see /usr/share/common-licenses/MIT
" | save -f $"($tui_deb_root)/usr/share/doc/gitkraft-tui/copyright"

    run-external "dpkg-deb" "--build" $tui_deb_root $"($dist_dir)/gitkraft-tui_($version)_($arch).deb"
    print $"✅ Built gitkraft-tui_($version)_($arch).deb"

    # ── .deb for gitkraft (GUI) ──────────────────────────────────────────────
    let gui_deb_root = $"($dist_dir)/deb-gui"
    mkdir $"($gui_deb_root)/DEBIAN"
    mkdir $"($gui_deb_root)/usr/bin"
    mkdir $"($gui_deb_root)/usr/share/doc/gitkraft"

    cp $"target/($target)/release/gitkraft" $"($gui_deb_root)/usr/bin/gitkraft"

    (gui-deb-control $version $arch) | save -f $"($gui_deb_root)/DEBIAN/control"

    $"GitKraft ($version)
Copyright 2024 Sorin Irimies
MIT License — see /usr/share/common-licenses/MIT
" | save -f $"($gui_deb_root)/usr/share/doc/gitkraft/copyright"

    run-external "dpkg-deb" "--build" $gui_deb_root $"($dist_dir)/gitkraft_($version)_($arch).deb"
    print $"✅ Built gitkraft_($version)_($arch).deb"

    # ── .rpm for gitkraft-tui ────────────────────────────────────────────────
    # Pre-compute the changelog date so the "..." inside format date does not
    # terminate the enclosing $"..." string interpolation prematurely.
    let rpm_date = (date now | format date "%a %b %d %Y")

    let rpm_build = $"($dist_dir)/rpmbuild"
    mkdir $"($rpm_build)/BUILD"
    mkdir $"($rpm_build)/RPMS"
    mkdir $"($rpm_build)/SOURCES"
    mkdir $"($rpm_build)/SPECS"
    mkdir $"($rpm_build)/SRPMS"

    $"Name:           gitkraft-tui
Version:        ($version)
Release:        1%{?dist}
Summary:        Terminal Git IDE written in Rust
License:        MIT
URL:            https://github.com/sorinirimies/gitkraft
BuildArch:      ($rpm_arch)

%description
A keyboard-driven terminal UI for Git, built on Ratatui.

%install
mkdir -p %{buildroot}/usr/bin
install -m 755 %{_sourcedir}/gitkraft-tui %{buildroot}/usr/bin/gitkraft-tui

%files
/usr/bin/gitkraft-tui

%changelog
* ($rpm_date) Sorin Irimies <sorinirimies@gmail.com> - ($version)-1
- Release ($version)
" | save -f $"($rpm_build)/SPECS/gitkraft-tui.spec"

    cp $"target/($target)/release/gitkraft-tui" $"($rpm_build)/SOURCES/gitkraft-tui"

    run-external "rpmbuild" ...(rpmbuild-args $rpm_build $"($rpm_build)/SPECS/gitkraft-tui.spec")

    let rpm_file = (ls $"($rpm_build)/RPMS/($rpm_arch)/*.rpm" | first).name
    cp $rpm_file $"($dist_dir)/gitkraft-tui-($version)-($rpm_arch).rpm"
    print $"✅ Built gitkraft-tui-($version)-($rpm_arch).rpm"

    # ── .rpm for gitkraft (GUI) ──────────────────────────────────────────────
    (gui-rpm-spec $version $rpm_arch $rpm_date) | save -f $"($rpm_build)/SPECS/gitkraft.spec"

    cp $"target/($target)/release/gitkraft" $"($rpm_build)/SOURCES/gitkraft"

    run-external "rpmbuild" ...(rpmbuild-args $rpm_build $"($rpm_build)/SPECS/gitkraft.spec")

    let gui_rpm_file = (ls $"($rpm_build)/RPMS/($rpm_arch)/gitkraft-[0-9]*.rpm" | first).name
    cp $gui_rpm_file $"($dist_dir)/gitkraft-($version)-($rpm_arch).rpm"
    print $"✅ Built gitkraft-($version)-($rpm_arch).rpm"

    print ""
    print "📦 Linux packages:"
    ls $dist_dir | where { |it| is-linux-package-file ($it.name | path basename) } | select name size | print
}
