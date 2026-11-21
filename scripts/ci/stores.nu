#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Package-manager manifests for a release
# ──────────────────────────────────────────────────────────────────────────────
# Renders, from the release files in `dist/` (their sha256 is taken from the files
# themselves, so nothing is downloaded again):
#   <out>/homebrew/Casks/gitkraft.rb       the GUI app, from the universal DMG
#   <out>/homebrew/Formula/<bin>.rb        every other binary, from the tarballs
#   <out>/scoop/bucket/gitkraft.json      every binary, from the Windows zips
#   <out>/aur-bin/PKGBUILD                 gitkraft-bin, from the .deb files
# The release workflow pushes them with push_manifest.nu (Homebrew tap, Scoop
# bucket) and the AUR action. Edit APP and BINS in scripts/project.nu, not here.
#
# Usage: nu scripts/ci/stores.nu manifests <version> [--dist dist] [--out manifests]
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [APP BINS]
use package_desktop.nu [plain-version deb-name deb-arch dmg-name]

export const TAP = "sorinirimies/homebrew-tap"
export const BUCKET = "sorinirimies/scoop-bucket"
const WINDOWS = "x86_64-pc-windows-msvc"
# The targets a Homebrew formula installs from, keyed by Homebrew's OS and CPU blocks.
const BREW_TARGETS = [
    [os, cpu, target];
    [macos, arm, aarch64-apple-darwin]
    [macos, intel, x86_64-apple-darwin]
    [linux, arm, aarch64-unknown-linux-gnu]
    [linux, intel, x86_64-unknown-linux-gnu]
]
const AUR_ARCHES = [[arch, target]; [x86_64, x86_64-unknown-linux-gnu] [aarch64, aarch64-unknown-linux-gnu]]

# The download URL of a release file.
export def asset-url [app: record, version: string, file: string]: nothing -> string {
    $"($app.homepage)/releases/download/v(plain-version $version)/($file)"
}

# A Homebrew class name: `gitkraft-tui` → `GitkraftTui`.
export def brew-class [name: string]: nothing -> string {
    $name | split row --regex '[-_]' | each {|w| $w | str capitalize } | str join
}

# A description as Homebrew's lint wants it: without a leading "<name> …:" and
# without a leading article ("GitKraft TUI: terminal Git IDE" → "Terminal Git IDE").
export def brew-desc [text: string, name: string]: nothing -> string {
    $text | str replace --regex $"^\(?i\)($name)[^:]*:\\s*" '' | str replace --regex '^(?i)(a|an|the)\s+' '' | str capitalize
}

# The formula of a command-line binary; `shas` maps each tarball name to its sha256.
export def brew-formula [app: record, bin: record, version: string, shas: record]: nothing -> string {
    let block = {|os|
        let rows = $BREW_TARGETS | where os == $os | each {|t|
            let file = $"($bin.bin)-($t.target).tar.gz"
            $"    on_($t.cpu) do\n      url \"(asset-url $app $version $file)\"\n      sha256 \"($shas | get $file)\"\n    end"
        }
        $"  on_($os) do\n($rows | str join "\n")\n  end"
    }
    $"# Rendered by scripts/ci/stores.nu and pushed to ($TAP).
class (brew-class $bin.bin) < Formula
  desc \"(brew-desc $bin.summary $app.title)\"
  homepage \"($app.homepage)\"
  version \"(plain-version $version)\"
  license \"($app.license)\"

(do $block macos)

(do $block linux)

  def install
    bin.install \"($bin.bin)\"
  end

  test do
    assert_match version.to_s, shell_output\(\"#{bin}/($bin.bin) --version\")
  end
end
"
}

# The cask of the desktop app (the universal DMG). The app is signed ad hoc, not
# notarized, so macOS refuses a quarantined copy: the caveat says how to clear it.
export def brew-cask [app: record, version: string, sha: string]: nothing -> string {
    let url = asset-url $app $version (dmg-name $app $version) | str replace --all (plain-version $version) '#{version}'
    $"# Rendered by scripts/ci/stores.nu and pushed to ($TAP).
cask \"($app.id)\" do
  version \"(plain-version $version)\"
  sha256 \"($sha)\"

  url \"($url)\"
  name \"($app.title)\"
  desc \"(brew-desc $app.summary $app.title)\"
  homepage \"($app.homepage)\"

  depends_on :macos

  app \"($app.title).app\"

  zap trash: \"~/Library/Application Support/($app.id)\"

  caveats <<~EOS
    ($app.title) is not notarized by Apple. If macOS says it cannot be opened, run:
      xattr -dr com.apple.quarantine \"#{appdir}/($app.title).app\"
  EOS
end
"
}

# One Scoop manifest for every binary; `shas` maps each zip name to its sha256.
export def scoop-manifest [app: record, bins: list<record>, version: string, shas: record]: nothing -> string {
    let files = $bins | each {|b| $"($b.bin)-($WINDOWS).zip" }
    let gui = $bins | where kind == gui | each {|b| [$"($b.bin).exe" $app.title] }
    {
        version: (plain-version $version)
        description: $app.summary
        homepage: $app.homepage
        license: $app.license
        architecture: {
            "64bit": {
                url: ($files | each {|f| asset-url $app $version $f })
                hash: ($files | each {|f| $shas | get $f })
            }
        }
        bin: ($bins | each {|b| $"($b.bin).exe" })
        shortcuts: $gui
        checkver: {github: $app.homepage}
        autoupdate: {
            architecture: {
                "64bit": {url: ($files | each {|f| $"($app.homepage)/releases/download/v$version/($f)" })}
            }
        }
    } | to json --indent 4 | $in + "\n"
}

# The .deb file of a binary.
export def deb-file [bin: record, version: string, target: string]: nothing -> string {
    $"(deb-name $bin.bin)_(plain-version $version)_(deb-arch $target).deb"
}

# The gitkraft-bin PKGBUILD: the release's .deb files unpacked (binaries, desktop
# entries, metainfo, icons), no compiler needed. `shas` maps each .deb and
# `LICENSE` to its sha256.
export def aur-bin [app: record, bins: list<record>, version: string, shas: record]: nothing -> string {
    let v = plain-version $version
    let per_arch = $AUR_ARCHES | each {|a|
        let debs = $bins | each {|b| deb-file $b $version $a.target }
        let sources = $debs | each {|d| $"\"($d)::(asset-url $app $version $d)\"" | str replace --all $v '${pkgver}' } | str join "\n    "
        let sums = $debs | each {|d| $"'($shas | get $d)'" } | str join "\n    "
        $"source_($a.arch)=\(\n    ($sources)\n)\nsha256sums_($a.arch)=\(\n    ($sums)\n)"
    } | str join "\n"
    let noextract = $AUR_ARCHES | each {|a| $bins | each {|b| deb-file $b $version $a.target | str replace $v '${pkgver}' } } | flatten | each {|d| $"\"($d)\"" } | str join " "
    let raw = $app.homepage | str replace "https://github.com/" "https://raw.githubusercontent.com/"
    $"# Maintainer: ($app.maintainer)
# Generated by scripts/ci/stores.nu from the release's .deb files.
pkgname=($app.id)-bin
pkgver=($v)
pkgrel=1
pkgdesc=\"($app.summary) \(prebuilt)\"
arch=\('x86_64' 'aarch64')
url=\"($app.homepage)\"
license=\('($app.license)')
depends=\('gcc-libs' 'glibc' 'zlib' 'libxkbcommon' 'wayland' 'libx11' 'vulkan-icd-loader' 'fontconfig')
provides=\('($app.id)')
conflicts=\('($app.id)')
options=\('!strip' '!debug')
source=\(\"LICENSE-${pkgver}::($raw)/v${pkgver}/LICENSE\")
sha256sums=\('($shas | get LICENSE)')
($per_arch)
noextract=\(($noextract))

package\() {
    local deb
    for deb in \"$srcdir\"/*.deb; do
        bsdtar -xOf \"$deb\" 'data.tar.*' | bsdtar -xf - -C \"$pkgdir\" --no-same-owner
    done
    chmod 755 \"$pkgdir\"
    rm -rf \"$pkgdir/usr/share/doc\"
    install -Dm644 \"$srcdir/LICENSE-${pkgver}\" \"$pkgdir/usr/share/licenses/$pkgname/LICENSE\"
}
"
}

# sha256 of every file in `dir`, keyed by file name.
def hashes [dir: string]: nothing -> record {
    ls $dir | where type == file | reduce --fold {} {|f, acc| $acc | insert ($f.name | path basename) (open --raw $f.name | hash sha256) }
}

def "main manifests" [version: string, --dist: string = "dist", --out: string = "manifests"] {
    let shas = hashes $dist | insert LICENSE (open --raw LICENSE | hash sha256)
    let gui = $BINS | where kind == gui
    let tools = $BINS | where kind != gui
    mkdir $"($out)/homebrew/Casks" $"($out)/homebrew/Formula" $"($out)/scoop/bucket" $"($out)/aur-bin"
    if ($gui | is-not-empty) {
        brew-cask $APP $version ($shas | get (dmg-name $APP $version)) | save --force $"($out)/homebrew/Casks/($APP.id).rb"
    }
    for b in $tools {
        brew-formula $APP $b $version $shas | save --force $"($out)/homebrew/Formula/($b.bin).rb"
    }
    scoop-manifest $APP $BINS $version $shas | save --force $"($out)/scoop/bucket/($APP.id).json"
    aur-bin $APP $BINS $version $shas | save --force $"($out)/aur-bin/PKGBUILD"
    print (do { cd $out; ls **/* | where type == file | get name | sort | str join "\n" })
}

def main [] {
    print "usage: nu scripts/ci/stores.nu manifests <version> [--dist dist] [--out manifests]"
}
