#!/usr/bin/env nu
# ── GitKraft · test_apt_repo.nu ─────────────────────────────────────
# The apt repository: index and Release formats, the user-facing source and
# commands, and a whole repository built and signed from two tiny .debs.

use std/assert
use runner.nu *
use ../project.nu [APP BINS]
use ../ci/apt_repo.nu *

const SCRIPT = path self ../ci/apt_repo.nu

def app [] {
    {id: "demo", title: "Demo", summary: "A demo", maintainer: "Me <me@example.com>", homepage: "https://github.com/me/demo"}
}

def "test apt_repo: pages url from the homepage" [] {
    assert equal (site-url (app)) "https://me.github.io/demo"
    assert equal (site-url $APP) "https://sorinirimies.github.io/gitkraft"
}

def "test apt_repo: only a version tag is served as given" [] {
    assert equal (release-tag v1.6.1) v1.6.1
    assert equal (release-tag main) "" "a branch means the latest release"
    assert equal (release-tag "") ""
}

def "test apt_repo: a stanza is the control plus location and hashes" [] {
    let s = stanza "Package: demo\nVersion: 1.0\n\n" pool/main/demo.deb 42 m5 s256
    assert equal $s "Package: demo\nVersion: 1.0\nFilename: pool/main/demo.deb\nSize: 42\nMD5sum: m5\nSHA256: s256\n"
    assert equal (packages-index [$s $s]) $"($s)\n($s)" "stanzas are separated by one blank line"
}

def "test apt_repo: control fields are read by name" [] {
    let c = "Package: demo\nArchitecture: arm64\nDescription: x\n y"
    assert equal (field $c Architecture) arm64
    assert equal (field $c Package) demo
}

def "test apt_repo: release lists every index with size and hashes" [] {
    let files = [{path: main/binary-amd64/Packages, size: 10, md5: a, sha256: b} {path: main/binary-amd64/Packages.gz, size: 5, md5: c, sha256: d}]
    let r = release-file (app) [amd64 arm64] $files "Sat, 01 Jan 2000 00:00:00 UTC"
    assert ($r | str contains "Suite: stable\nCodename: stable\nDate: Sat, 01 Jan 2000 00:00:00 UTC\nArchitectures: amd64 arm64\nComponents: main\n")
    assert ($r | str contains "SHA256:\n b 10 main/binary-amd64/Packages\n d 5 main/binary-amd64/Packages.gz\n")
    assert ($r | str contains "MD5Sum:\n a 10 main/binary-amd64/Packages\n")
}

def "test apt_repo: users get a signed-by source and one install line" [] {
    let s = sources-file (app)
    assert equal $s "Types: deb\nURIs: https://me.github.io/demo\nSuites: stable\nComponents: main\nSigned-By: /etc/apt/keyrings/demo.gpg\n"
    let c = install-commands $APP $BINS
    assert ($c | str contains "/etc/apt/keyrings/gitkraft.gpg")
    assert ($c | str ends-with "sudo apt install gitkraft gitkraft-tui gitkraft-web gitkraft-mcp")
    let page = index-html (app) [{bin: demo_cli, summary: "a <cli>"}] 1.0 ABC
    assert ($page | str contains "<code>demo-cli</code>: a &lt;cli&gt;")
    assert ($page | str contains "<code>ABC</code>")
}

# A .deb with just a control file, for package arch.
def tiny-deb [dir: string, name: string, arch: string] {
    let root = $"($dir)/root-($name)-($arch)"
    mkdir $"($root)/DEBIAN"
    $"Package: ($name)\nVersion: 1.0.0\nArchitecture: ($arch)\nMaintainer: Me <me@example.com>\nDescription: d\n" | save $"($root)/DEBIAN/control"
    ^dpkg-deb --root-owner-group --build $root $"($dir)/debs/($name)_1.0.0_($arch).deb" | ignore
}

def "test apt_repo: a built repository is complete, consistent and signed" [] {
    if (which dpkg-deb gpg gpgv | length) < 3 { print "  (skipped: needs dpkg-deb, gpg and gpgv)"; return }
    let d = mktemp -d
    mkdir $"($d)/debs"
    tiny-deb $d demo amd64
    tiny-deb $d demo arm64
    tiny-deb $d demo-tui amd64
    let home = $"($d)/gnupg"
    with-env {GNUPGHOME: $home, APT_SIGNING_KEY: ""} {
        let out = ^nu $SCRIPT import | lines | parse "{k}={v}" | transpose -rd
        assert equal $out.publish "false" "no secret, no publishing"
        ^nu $SCRIPT build $"($d)/debs" $"($d)/site" --key $out.fingerprint --date "Sat, 01 Jan 2000 00:00:00 UTC" err> /dev/null
        let suite = $"($d)/site/dists/stable"
        let release = open --raw $"($suite)/Release"
        assert ($release | str contains "Architectures: amd64 arm64")
        for a in [amd64 arm64] {
            let rel = $"main/binary-($a)/Packages"
            let bytes = open --raw $"($suite)/($rel)" | into binary
            assert ($release | str contains $" ($bytes | hash sha256) ($bytes | bytes length) ($rel)\n") $"($a) Packages hash"
            assert (($"($suite)/($rel).gz") | path exists)
        }
        let amd = open --raw $"($suite)/main/binary-amd64/Packages"
        assert equal ($amd | lines | where {|l| $l | str starts-with "Package:" } | sort) ["Package: demo" "Package: demo-tui"]
        let deb = open --raw $"($d)/site/pool/main/demo_1.0.0_amd64.deb" | into binary
        assert ($amd | str contains $"Filename: pool/main/demo_1.0.0_amd64.deb\nSize: ($deb | bytes length)\n")
        assert ($amd | str contains $"SHA256: ($deb | hash sha256)")
        # The check apt makes: the signature against the published key only.
        let ok = do { ^gpgv --keyring $"($d)/site/gitkraft.gpg" $"($suite)/Release.gpg" $"($suite)/Release" } | complete
        assert equal $ok.exit_code 0 "Release.gpg verifies against the published key"
        let inl = do { ^gpgv --keyring $"($d)/site/gitkraft.gpg" $"($suite)/InRelease" } | complete
        assert equal $inl.exit_code 0 "InRelease verifies against the published key"
        for f in [gitkraft.sources gitkraft.asc index.html .nojekyll] { assert ($"($d)/site/($f)" | path exists) $f }
    }
    rm -rf $d
}

def main [] { run-tests }
