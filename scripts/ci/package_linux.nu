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

    $"Package: gitkraft
Version: ($version)
Architecture: ($arch)
Maintainer: Sorin Irimies <sorinirimies@gmail.com>
Description: GitKraft — desktop GUI Git IDE written in Rust
 A mouse-driven desktop GUI for Git, built on Iced (Elm Architecture).
Homepage: https://github.com/sorinirimies/gitkraft
Depends: libxkbcommon0, libwayland-client0, libgl1
" | save -f $"($gui_deb_root)/DEBIAN/control"

    $"GitKraft ($version)
Copyright 2024 Sorin Irimies
MIT License — see /usr/share/common-licenses/MIT
" | save -f $"($gui_deb_root)/usr/share/doc/gitkraft/copyright"

    run-external "dpkg-deb" "--build" $gui_deb_root $"($dist_dir)/gitkraft_($version)_($arch).deb"
    print $"✅ Built gitkraft_($version)_($arch).deb"

    # ── .rpm for gitkraft-tui ────────────────────────────────────────────────
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
* (date now | format date "%a %b %d %Y") Sorin Irimies <sorinirimies@gmail.com> - ($version)-1
- Release ($version)
" | save -f $"($rpm_build)/SPECS/gitkraft-tui.spec"

    cp $"target/($target)/release/gitkraft-tui" $"($rpm_build)/SOURCES/gitkraft-tui"

    run-external "rpmbuild" "-bb"
        $"--define" $"_topdir (pwd)/($rpm_build)"
        $"($rpm_build)/SPECS/gitkraft-tui.spec"

    let rpm_file = (ls $"($rpm_build)/RPMS/($rpm_arch)/*.rpm" | first).name
    cp $rpm_file $"($dist_dir)/gitkraft-tui-($version)-($rpm_arch).rpm"
    print $"✅ Built gitkraft-tui-($version)-($rpm_arch).rpm"

    # ── .rpm for gitkraft (GUI) ──────────────────────────────────────────────
    $"Name:           gitkraft
Version:        ($version)
Release:        1%{?dist}
Summary:        Desktop GUI Git IDE written in Rust
License:        MIT
URL:            https://github.com/sorinirimies/gitkraft
BuildArch:      ($rpm_arch)
Requires:       libxkbcommon, wayland-libs-client, mesa-libGL

%description
A mouse-driven desktop GUI for Git, built on Iced (Elm Architecture).

%install
mkdir -p %{buildroot}/usr/bin
install -m 755 %{_sourcedir}/gitkraft %{buildroot}/usr/bin/gitkraft

%files
/usr/bin/gitkraft

%changelog
* (date now | format date "%a %b %d %Y") Sorin Irimies <sorinirimies@gmail.com> - ($version)-1
- Release ($version)
" | save -f $"($rpm_build)/SPECS/gitkraft.spec"

    cp $"target/($target)/release/gitkraft" $"($rpm_build)/SOURCES/gitkraft"

    run-external "rpmbuild" "-bb"
        $"--define" $"_topdir (pwd)/($rpm_build)"
        $"($rpm_build)/SPECS/gitkraft.spec"

    let gui_rpm_file = (ls $"($rpm_build)/RPMS/($rpm_arch)/gitkraft-[0-9]*.rpm" | first).name
    cp $gui_rpm_file $"($dist_dir)/gitkraft-($version)-($rpm_arch).rpm"
    print $"✅ Built gitkraft-($version)-($rpm_arch).rpm"

    print ""
    print "📦 Linux packages:"
    ls $dist_dir | where name =~ "(\.deb|\.rpm)" | select name size | print
}
