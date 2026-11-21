#!/usr/bin/env nu
# ── GitKraft · apt_repo.nu ──────────────────────────────────────────
# A signed apt repository from the release's .deb files, served by GitHub Pages:
#
#   <site>/pool/main/<file>.deb
#   <site>/dists/stable/main/binary-<arch>/Packages(.gz)
#   <site>/dists/stable/{Release,InRelease,Release.gpg}
#   <site>/<id>.gpg, <id>.asc, <id>.sources, index.html
#
# Only the latest release is served (Pages has no room for every version, and apt
# only installs the newest anyway).
#
# Usage:
#   nu scripts/ci/apt_repo.nu fetch [tag] [--dir debs]  # the release's .debs (gh)
#   nu scripts/ci/apt_repo.nu build <debs-dir> <site> [--key <fingerprint>]
#   nu scripts/ci/apt_repo.nu key --out <file>      # once: a dedicated signing key
#   nu scripts/ci/apt_repo.nu import                # CI: $APT_SIGNING_KEY → fingerprint
#   nu scripts/ci/apt_repo.nu verify <site> <image> # CI: install from it in a container

use ../project.nu [APP BINS]

export const SUITE = "stable"
export const COMPONENT = "main"

# Where Pages serves the repository: https://<owner>.github.io/<repo>.
export def site-url [app: record]: nothing -> string {
    let parts = $app.homepage | str replace --regex '^https://github.com/' '' | split row '/'
    $"https://($parts.0).github.io/($parts.1)"
}

# One Packages stanza: the package's control fields, then where it is and its hashes.
export def stanza [control: string, file: string, size: int, md5: string, sha256: string]: nothing -> string {
    let fields = $control | lines | where {|l| ($l | str trim) != "" } | str join "\n"
    $"($fields)\nFilename: ($file)\nSize: ($size)\nMD5sum: ($md5)\nSHA256: ($sha256)\n"
}

# A Packages index: stanzas separated by a blank line.
export def packages-index [stanzas: list<string>]: nothing -> string {
    $stanzas | str join "\n"
}

# The value of a control field ("Architecture: amd64" → amd64).
export def field [control: string, name: string]: nothing -> string {
    $control | lines | where {|l| $l | str starts-with $"($name):" } | first | str replace $"($name):" '' | str trim
}

# The Release file over the index files, given as {path (relative to the suite dir),
# size, md5, sha256}.
export def release-file [app: record, archs: list<string>, files: list<record>, date: string]: nothing -> string {
    let md5 = $files | each {|f| $" ($f.md5) ($f.size) ($f.path)" } | str join "\n"
    let sha = $files | each {|f| $" ($f.sha256) ($f.size) ($f.path)" } | str join "\n"
    [
        $"Origin: ($app.title)"
        $"Label: ($app.title)"
        $"Suite: ($SUITE)"
        $"Codename: ($SUITE)"
        $"Date: ($date)"
        $"Architectures: ($archs | str join ' ')"
        $"Components: ($COMPONENT)"
        $"Description: ($app.title) packages \(($app.homepage))"
        "MD5Sum:"
        $md5
        "SHA256:"
        $sha
    ] | str join "\n" | $in + "\n"
}

# The deb822 source users drop into /etc/apt/sources.list.d.
export def sources-file [app: record]: nothing -> string {
    $"Types: deb
URIs: (site-url $app)
Suites: ($SUITE)
Components: ($COMPONENT)
Signed-By: /etc/apt/keyrings/($app.id).gpg
"
}

# The commands that add the repository and install every package.
export def install-commands [app: record, bins: list<record>]: nothing -> string {
    let url = site-url $app
    let pkgs = $bins | get bin | str lowercase | str replace --all '_' '-' | str join ' '
    $"sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL ($url)/($app.id).gpg | sudo tee /etc/apt/keyrings/($app.id).gpg > /dev/null
curl -fsSL ($url)/($app.id).sources | sudo tee /etc/apt/sources.list.d/($app.id).sources > /dev/null
sudo apt update
sudo apt install ($pkgs)"
}

def escape-html [s: string]: nothing -> string {
    $s | str replace --all '&' '&amp;' | str replace --all '<' '&lt;' | str replace --all '>' '&gt;'
}

# The page Pages shows at the repository URL.
export def index-html [app: record, bins: list<record>, version: string, fingerprint: string]: nothing -> string {
    let rows = $bins | each {|b| $"<li><code>($b.bin | str lowercase | str replace --all '_' '-')</code>: (escape-html $b.summary)</li>" } | str join "\n"
    $"<!doctype html>
<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">
<title>($app.title) apt repository</title>
<style>body{font:16px/1.5 system-ui,sans-serif;max-width:46rem;margin:3rem auto;padding:0 1rem}pre{background:#111;color:#eee;padding:1rem;overflow-x:auto;border-radius:6px}</style>
</head><body>
<h1>($app.title) apt repository</h1>
<p>(escape-html $app.summary). Version ($version), for amd64 and arm64: Debian 12+, Ubuntu 22.04+, Pop!_OS, Linux Mint 21+.</p>
<pre>(escape-html (install-commands $app $bins))</pre>
<p>Packages:</p><ul>
($rows)
</ul>
<p>Signing key fingerprint: <code>($fingerprint)</code>. Source: <a href=\"($app.homepage)\">($app.homepage)</a>.</p>
</body></html>
"
}

def file-hashes [path: string, rel: string]: nothing -> record {
    let bytes = open --raw $path | into binary
    {path: $rel, size: ($bytes | bytes length), md5: ($bytes | hash md5), sha256: ($bytes | hash sha256)}
}

# Signs Release into InRelease (inline) and Release.gpg (detached).
def sign [dir: string, key: string] {
    ^gpg --batch --yes --local-user $key --digest-algo SHA512 --clearsign --output $"($dir)/InRelease" $"($dir)/Release"
    ^gpg --batch --yes --local-user $key --digest-algo SHA512 --armor --detach-sign --output $"($dir)/Release.gpg" $"($dir)/Release"
}

# The tag to serve: a vX.Y.Z tag as given, anything else (empty, or the branch a
# workflow_run reports for a release started by hand) means the latest release.
export def release-tag [given: string]: nothing -> string {
    if ($given =~ '^v\d') { $given } else { "" }
}

# Downloads the .deb files of a release (default: the latest) into --dir.
def "main fetch" [tag: string = "", --dir: string = "debs"] {
    let t = release-tag $tag
    mkdir $dir
    let repo = $APP.homepage | str replace 'https://github.com/' ''
    if ($t | is-empty) { ^gh release download -R $repo -p '*.deb' -D $dir } else { ^gh release download $t -R $repo -p '*.deb' -D $dir }
    print -e $"✅ (glob $'($dir)/*.deb' | length) .deb files of ($t | default -e 'the latest release')"
}

# The repository in <site> from every .deb in <debs>.
def "main build" [debs: string, site: string, --key: string, --date: string] {
    let files = glob $"($debs)/*.deb" | sort
    if ($files | is-empty) { error make {msg: $"no .deb files in ($debs)"} }
    let pool = $"pool/($COMPONENT)"
    mkdir $"($site)/($pool)"
    let pkgs = $files | each {|f|
        let name = $f | path basename
        cp $f $"($site)/($pool)/($name)"
        let control = ^dpkg-deb --field $f
        let h = file-hashes $f $"($pool)/($name)"
        {arch: (field $control Architecture), version: (field $control Version), stanza: (stanza $control $h.path $h.size $h.md5 $h.sha256)}
    }
    let archs = $pkgs | get arch | uniq | sort
    let suite = $"($site)/dists/($SUITE)"
    let indexes = $archs | each {|a|
        let rel = $"($COMPONENT)/binary-($a)/Packages"
        mkdir $"($suite)/($COMPONENT)/binary-($a)"
        packages-index ($pkgs | where arch == $a | get stanza) | save --force $"($suite)/($rel)"
        ^gzip -9nkf $"($suite)/($rel)"
        [(file-hashes $"($suite)/($rel)" $rel) (file-hashes $"($suite)/($rel).gz" $"($rel).gz")]
    } | flatten
    let when = $date | default (date now | date to-timezone UTC | format date "%a, %d %b %Y %H:%M:%S UTC")
    release-file $APP $archs $indexes $when | save --force $"($suite)/Release"
    sources-file $APP | save --force $"($site)/($APP.id).sources"
    let version = $pkgs | get version | uniq | str join ", "
    let fingerprint = if $key == null { "unsigned" } else {
        sign $suite $key
        ^gpg --batch --yes --export --output $"($site)/($APP.id).gpg" $key
        ^gpg --batch --yes --armor --export --output $"($site)/($APP.id).asc" $key
        $key
    }
    index-html $APP $BINS $version $fingerprint | save --force $"($site)/index.html"
    "" | save --force $"($site)/.nojekyll"
    print -e $"✅ apt repository in ($site): ($pkgs | length) packages, ($archs | str join ' ')"
}

# A new signing key (ed25519, no passphrase, no expiry) in a throwaway keyring,
# written armored to --out. Keep that file safe (it is the only copy), then:
#   gh secret set APT_SIGNING_KEY -R <owner>/<repo> < <out>
def "main key" [--out: string, --name: string] {
    let out = $out | default $"($APP.id)-apt-signing-key.asc"
    if ($out | path exists) { error make {msg: $"($out) exists; not overwriting a key"} }
    let home = mktemp -d
    let uid = $"($name | default $'($APP.title) apt repository') <($APP.maintainer | parse --regex '<(?<m>[^>]+)>' | get m.0)>"
    with-env {GNUPGHOME: $home} {
        ^gpg --batch --passphrase '' --quick-generate-key $uid ed25519 sign never
        ^gpg --batch --armor --export-secret-keys --output $out $uid
        let fpr = ^gpg --batch --with-colons --list-keys $uid | lines | where {|l| $l | str starts-with "fpr:" } | first | split row ':' | get 9
        print $"Key ($fpr) written to ($out). Now: gh secret set APT_SIGNING_KEY < ($out)"
    }
    rm -rf $home
}

# Imports $APT_SIGNING_KEY into $GNUPGHOME and prints `fingerprint=<fpr>` for
# $GITHUB_OUTPUT; without the secret, a throwaway key (the repository is then
# verified but not published) and `publish=false`.
def "main import" [] {
    if ($env.GNUPGHOME? | is-not-empty) { mkdir $env.GNUPGHOME; ^chmod 700 $env.GNUPGHOME }
    let secret = $env.APT_SIGNING_KEY? | default ""
    if ($secret | is-empty) {
        ^gpg --batch --passphrase '' --quick-generate-key $"($APP.title) apt test <test@invalid>" ed25519 sign 1d
    } else {
        $secret | ^gpg --batch --import
    }
    let fpr = ^gpg --batch --with-colons --list-secret-keys | lines | where {|l| $l | str starts-with "fpr:" } | first | split row ':' | get 9
    print $"fingerprint=($fpr)"
    print $"publish=(not ($secret | is-empty))"
}

# Adds <site> as a source inside a fresh <image> container, the way users do (key
# file + deb822 source, signature checked), installs every package and runs the
# command-line ones.
def "main verify" [site: string, image: string] {
    let pkgs = $BINS | get bin | str lowercase | str replace --all '_' '-'
    let runs = $BINS | where kind != gui | each {|b| $"($b.bin) --version" } | str join " && "
    let script = [
        "set -e"
        "export DEBIAN_FRONTEND=noninteractive"
        "install -d -m 0755 /etc/apt/keyrings"
        $"cp /repo/($APP.id).gpg /etc/apt/keyrings/($APP.id).gpg"
        $"sed 's#^URIs: .*#URIs: file:/repo#' /repo/($APP.id).sources > /etc/apt/sources.list.d/($APP.id).sources"
        "apt-get update"
        $"apt-get install -y --no-install-recommends ($pkgs | str join ' ')"
        $runs
    ] | str join "\n"
    ^docker run --rm -v $"($site | path expand):/repo:ro" $image sh -c $script
    print -e $"✅ ($image): installed ($pkgs | str join ', ') from the repository"
}

def main [] {
    print "usage: apt_repo.nu build <debs> <site> [--key <fpr>] | key --out <file> | import | verify <site> <image>"
}
