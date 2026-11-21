#!/usr/bin/env nu
# ── GitKraft · test_stores.nu ───────────────────────────────────────
# The Homebrew, Scoop and AUR (-bin) manifests and the manifest pusher, on a
# fixture app, plus the project's own APP and BINS.

use std/assert
use runner.nu *
use ../project.nu [APP BINS]
use ../ci/stores.nu *
use ../ci/push_manifest.nu [remote_url commit_message files_under]

def app [] {
    {id: "demo", title: "Demo", summary: "A demo app: does demo things", maintainer: "Me <me@example.com>", homepage: "https://github.com/me/demo", license: "MIT"}
}

def bins [] {
    [
        {crate: demo, bin: demo, kind: gui, summary: "Demo: the desktop app"}
        {crate: demo-tui, bin: demo-tui, kind: tui, summary: "Demo TUI: a terminal app"}
    ]
}

# Every file name a fixture manifest asks a hash for, hashed to its own name.
def shas [] {
    let v = "1.2.3"
    let tars = [aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu] | each {|t| $"demo-tui-($t).tar.gz" }
    let zips = [demo demo-tui] | each {|b| $"($b)-x86_64-pc-windows-msvc.zip" }
    let debs = [amd64 arm64] | each {|a| [$"demo_($v)_($a).deb" $"demo-tui_($v)_($a).deb"] } | flatten
    $tars ++ $zips ++ $debs ++ ["demo-1.2.3-macos.dmg" LICENSE] | reduce --fold {} {|f, acc| $acc | insert $f $"sha-of-($f)" }
}

def "test stores: homebrew class names are camel case" [] {
    assert equal (brew-class gitkraft-tui) GitkraftTui
    assert equal (brew-class demo_cli) DemoCli
    assert equal (brew-class demo) Demo
}

def "test stores: descriptions lose the name prefix and a leading article" [] {
    assert equal (brew-desc "Demo TUI: a terminal app" Demo) "Terminal app" "the name prefix and the article go"
    assert equal (brew-desc "A demo app: does demo things" Demo) "Demo app: does demo things" "a colon after other words stays"
    assert equal (brew-desc "The Git IDE" Demo) "Git IDE"
}

def "test stores: a formula installs the right tarball on each os and cpu" [] {
    let f = brew-formula (app) (bins | get 1) v1.2.3 (shas)
    assert ($f | str contains "class DemoTui < Formula")
    assert ($f | str contains "version \"1.2.3\"")
    assert ($f | str contains "desc \"Terminal app\"")
    for t in [[cpu target]; [arm aarch64-apple-darwin] [intel x86_64-apple-darwin]] {
        assert ($f | str contains $"on_($t.cpu) do\n      url \"https://github.com/me/demo/releases/download/v1.2.3/demo-tui-($t.target).tar.gz\"\n      sha256 \"sha-of-demo-tui-($t.target).tar.gz\"")
    }
    let linux = $f | str substring (($f | str index-of "on_linux")).. 
    assert ($linux | str contains "demo-tui-aarch64-unknown-linux-gnu.tar.gz") "linux arm"
    assert ($linux | str contains "demo-tui-x86_64-unknown-linux-gnu.tar.gz") "linux intel"
    assert ($f | str contains "bin.install \"demo-tui\"")
    assert ($f | str contains "#{bin}/demo-tui --version")
}

def "test stores: the cask installs the app from the versioned dmg and explains quarantine" [] {
    let c = brew-cask (app) 1.2.3 abc
    assert ($c | str contains "cask \"demo\" do\n  version \"1.2.3\"\n  sha256 \"abc\"")
    assert ($c | str contains "url \"https://github.com/me/demo/releases/download/v#{version}/demo-#{version}-macos.dmg\"")
    assert not ($c | str contains "1.2.3-macos") "the url follows the version stanza"
    assert ($c | str contains "depends_on :macos")
    assert ($c | str contains "app \"Demo.app\"")
    assert ($c | str contains "xattr -dr com.apple.quarantine")
}

def "test stores: one scoop manifest carries every binary and a shortcut for the gui" [] {
    let m = scoop-manifest (app) (bins) v1.2.3 (shas) | from json
    assert equal $m.version "1.2.3"
    assert equal $m.bin [demo.exe demo-tui.exe]
    assert equal $m.shortcuts [[demo.exe Demo]]
    let a = $m.architecture."64bit"
    assert equal ($a.url | length) 2
    assert equal $a.hash [sha-of-demo-x86_64-pc-windows-msvc.zip sha-of-demo-tui-x86_64-pc-windows-msvc.zip]
    assert equal ($m.autoupdate.architecture."64bit".url | first) "https://github.com/me/demo/releases/download/v$version/demo-x86_64-pc-windows-msvc.zip"
}

def "test stores: the -bin pkgbuild unpacks every deb of its architecture" [] {
    let p = aur-bin (app) (bins) v1.2.3 (shas)
    assert ($p | str contains "pkgname=demo-bin\npkgver=1.2.3\npkgrel=1\n")
    assert ($p | str contains "provides=('demo')\nconflicts=('demo')")
    assert ($p | str contains "\"demo-tui_${pkgver}_amd64.deb::https://github.com/me/demo/releases/download/v${pkgver}/demo-tui_${pkgver}_amd64.deb\"")
    assert ($p | str contains "source_aarch64=(\n    \"demo_${pkgver}_arm64.deb::")
    assert ($p | str contains "sha256sums_x86_64=(\n    'sha-of-demo_1.2.3_amd64.deb'\n    'sha-of-demo-tui_1.2.3_amd64.deb'\n)")
    assert ($p | str contains "sha256sums=('sha-of-LICENSE')")
    assert ($p | str contains "noextract=(\"demo_${pkgver}_amd64.deb\"")
    assert ($p | str contains "bsdtar -xOf \"$deb\" 'data.tar.*' | bsdtar -xf - -C \"$pkgdir\"")
    assert not ($p | str contains "1.2.3_amd64.deb::") "a new pkgver needs no other edit"
}

def "test stores: every project binary has a manifest" [] {
    let names = $BINS | get bin
    let tars = $BINS | each {|b| [aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu] | each {|t| $"($b.bin)-($t).tar.gz" } } | flatten
    let zips = $names | each {|b| $"($b)-x86_64-pc-windows-msvc.zip" }
    let debs = $BINS | each {|b| [amd64 arm64] | each {|a| deb-file $b 9.9.9 (if $a == amd64 { "x86_64-unknown-linux-gnu" } else { "aarch64-unknown-linux-gnu" }) } } | flatten
    let s = $tars ++ $zips ++ $debs ++ [LICENSE] | reduce --fold {} {|f, acc| $acc | insert $f x }
    let scoop = scoop-manifest $APP $BINS 9.9.9 $s | from json
    assert equal ($scoop.bin | length) ($names | length)
    assert equal ($scoop.shortcuts | length) ($BINS | where kind == gui | length)
    for b in ($BINS | where kind != gui) { brew-formula $APP $b 9.9.9 $s | ignore }
    let p = aur-bin $APP $BINS 9.9.9 $s
    assert ($p | str contains $"pkgname=($APP.id)-bin")
}

def "test push_manifest: token url, message and the files to copy" [] {
    assert equal (remote_url me/tap t0k) "https://x-access-token:t0k@github.com/me/tap.git"
    assert equal (commit_message gitkraft v1.2.3) "gitkraft: update to 1.2.3"
    let d = mktemp -d
    mkdir $"($d)/Casks" $"($d)/Formula"
    "x" | save $"($d)/Casks/a.rb"
    "y" | save $"($d)/Formula/b.rb"
    assert equal (files_under $d) [Casks/a.rb Formula/b.rb]
    rm -rf $d
}

def main [] { run-tests }
