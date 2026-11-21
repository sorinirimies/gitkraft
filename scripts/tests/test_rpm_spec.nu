#!/usr/bin/env nu
# ── GitKraft · test_rpm_spec.nu ─────────────────────────────────────
# The RPM spec shared by COPR and OBS, on a fixture app and on the project.

use std/assert
use runner.nu *
use ../project.nu [APP BINS]
use ../ci/rpm_spec.nu *

def app [] {
    {id: "demo", title: "Demo", summary: "A demo", maintainer: "Me <me@example.com>", homepage: "https://github.com/me/demo", license: "MIT", categories: [Development], desktop_ids: {demo: "io.me.Demo"}, icon: "io.me.Demo"}
}

def bins [] {
    [
        {crate: demo, bin: demo, kind: gui, summary: "Demo: the app"}
        {crate: demo-tui, bin: demo_tui, kind: tui, summary: "Demo TUI"}
        {crate: demo-mcp, bin: demo-mcp, kind: cli, summary: "Demo MCP"}
    ]
}

def "test rpm_spec: percent signs survive rpm macro expansion" [] {
    assert equal (rpm-escape "Exec=demo %U") "Exec=demo %%U"
}

def "test rpm_spec: the main binary is the app package, the others subpackages" [] {
    let s = rpm-spec (app) (bins) v1.2.3 ["assets/icons/demo.svg"]
    assert ($s | str contains "Name:           demo\nVersion:        1.2.3\n")
    assert ($s | str contains "Source0:        https://github.com/me/demo/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz")
    assert ($s | str contains "Source1:        %{name}-%{version}-vendor.tar.xz")
    assert ($s | str contains "%package -n demo-tui\nSummary:        Demo TUI")
    assert ($s | str contains "%files -n demo-tui\n%license LICENSE\n/usr/bin/demo_tui")
    assert ($s | str contains "%files\n%license LICENSE\n/usr/bin/demo\n")
    assert not ($s | str contains "%package -n demo\n") "the main binary is no subpackage"
    assert ($s | str contains "cargo build --release --frozen -p demo -p demo-tui -p demo-mcp")
}

def "test rpm_spec: install lines put every file where its deb puts it" [] {
    let s = rpm-spec (app) (bins) 1.2.3 ["assets/icons/demo.svg"]
    assert ($s | str contains "install -Dpm0755 target/release/demo %{buildroot}/usr/bin/demo\n")
    assert ($s | str contains "install -Dpm0644 assets/icons/demo.svg %{buildroot}/usr/share/icons/hicolor/scalable/apps/io.me.Demo.svg")
    assert ($s | str contains "install -Dpm0644 /dev/stdin %{buildroot}/usr/share/applications/io.me.Demo.desktop <<'EOF'\n[Desktop Entry]")
    assert ($s | str contains "/usr/share/metainfo/io.me.Demo.metainfo.xml")
    assert not ($s | str contains "target//") "no empty target triple in paths"
}

def "test rpm_spec: vendored crates are appended to the cargo config, offline" [] {
    let s = rpm-spec (app) (bins) 1.2.3 []
    assert ($s | str contains "cat >> .cargo/config.toml <<'EOF'") "the repo's own config stays"
    assert ($s | str contains "replace-with = \"vendored-sources\"")
    assert ($s | str contains "--frozen")
}

def "test rpm_spec: build dependencies resolve on fedora and opensuse alike" [] {
    let s = rpm-spec $APP $BINS 1.0.0 []
    for r in $RPM_BUILD_REQUIRES { assert ($s | str contains $"BuildRequires:  ($r)\n") $r }
    assert ($RPM_BUILD_REQUIRES | all {|r| ($r | str starts-with "pkgconfig(") or ($r | str starts-with "perl(") or ($r =~ '^(cargo|rust|gcc|make)\b') }) "only portable names"
}

def "test rpm_spec: zypper on opensuse, dnf elsewhere" [] {
    assert equal (install-command opensuse-tumbleweed [a]) [zypper --non-interactive install --no-recommends --force-resolution a]
    assert equal (install-command opensuse-leap [a] | first) zypper
    assert equal (install-command --local opensuse-tumbleweed [a.rpm]) [zypper --non-interactive install --no-recommends --force-resolution --allow-unsigned-rpm a.rpm]
    assert equal (install-command --local fedora [a.rpm]) [dnf install -y --setopt=install_weak_deps=False a.rpm]
    assert equal (install-command fedora [a b]) [dnf install -y --setopt=install_weak_deps=False a b]
}

def "test rpm_spec: copr config and chroots" [] {
    assert equal (copr-config l u t) "[copr-cli]\nlogin = l\nusername = u\ntoken = t\ncopr_url = https://copr.fedorainfracloud.org\n"
    assert ($COPR_CHROOTS | all {|c| $c =~ '^fedora-(rawhide|\d+)-(x86_64|aarch64)$' })
    assert equal (vendor-name $APP v1.6.0) "gitkraft-1.6.0-vendor.tar.xz"
}

def "test rpm_spec: obs project, repositories and the zypper url" [] {
    let m = obs-project-meta me (app)
    assert ($m | str starts-with "<project name=\"home:me:demo\">")
    assert ($m | str contains "<person userid=\"me\" role=\"maintainer\"/>")
    assert ($m | str contains "<repository name=\"openSUSE_Tumbleweed\">\n    <path project=\"openSUSE:Factory\" repository=\"snapshot\"/>\n    <arch>x86_64</arch>\n  </repository>")
    assert ($m | str contains "<path project=\"openSUSE:Factory:ARM\" repository=\"standard\"/>\n    <arch>aarch64</arch>")
    assert ((obs-package-meta me (app)) | str contains "<package name=\"demo\" project=\"home:me:demo\">")
    assert equal (obs-repo-url me (app) openSUSE_Tumbleweed) "https://download.opensuse.org/repositories/home:/me:/demo/openSUSE_Tumbleweed/home:me:demo.repo"
}

def main [] { run-tests }
