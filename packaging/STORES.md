# Where GitKraft is published

Every release (`just bump X.Y.Z`, then the tag push) runs `.github/workflows/release.yml`.
A store job whose secret is missing is skipped, so the release itself never fails on it.

| Channel | OS | What it ships | Job | Needs |
|---|---|---|---|---|
| GitHub release | all | tarballs/zips of the four binaries, `.deb`, `.rpm`, AppImage, universal DMG, Windows installer | `publish` | nothing |
| crates.io | all | `cargo install gitkraft` / `gitkraft-tui` / `gitkraft-web` / `gitkraft-mcp` | `crates-io` | `CRATES_IO_TOKEN` ✔ |
| Snap Store | Linux | `gitkraft`, `gitkraft-tui` | `publish-snap` | `SNAPCRAFT_STORE_CREDENTIALS` ✔ |
| Flathub (and so COSMIC Store, GNOME Software, KDE Discover) | Linux | the GUI | `update-flathub` | first submission, then `FLATHUB_TOKEN` |
| AUR `gitkraft` | Arch | built from the source tarball | `aur` | `AUR_USERNAME`, `AUR_EMAIL`, `AUR_SSH_PRIVATE_KEY` |
| AUR `gitkraft-bin` | Arch | the release `.deb`s unpacked, no compiling | `aur` | same |
| Homebrew tap | macOS, Linux | cask `gitkraft` (the app), formulae `gitkraft-tui`, `gitkraft-web`, `gitkraft-mcp` | `homebrew` | `TAP_GITHUB_TOKEN` |
| Scoop bucket | Windows | all four `.exe`s, Start-menu shortcut for the GUI | `scoop` | `TAP_GITHUB_TOKEN` |
| winget | Windows | the installer | `winget` | first submission, then `WINGET_TOKEN` |

Generators: `scripts/ci/stores.nu` (Homebrew, Scoop, `gitkraft-bin`), `scripts/ci/package_desktop.nu aur`
(`gitkraft`), `scripts/ci/update_flathub.nu`, `scripts/ci/snap_prepare.nu`. Tests: `scripts/tests/`.
Try them on a real release: `gh release download vX.Y.Z -D /tmp/rel`, then
`nu scripts/ci/stores.nu manifests X.Y.Z --dist /tmp/rel --out /tmp/manifests`.

## One-time steps (the owner's accounts)

**Homebrew and Scoop.** The repos `sorinirimies/homebrew-tap` and `sorinirimies/scoop-bucket` exist
(nuance uses them). Create a fine-grained token: only those two repositories, *Contents: read and
write*. Save it as `TAP_GITHUB_TOKEN` in gitkraft (and in nuance, which expects the same name):

    gh secret set TAP_GITHUB_TOKEN -R sorinirimies/gitkraft

**AUR.** Make an account on aur.archlinux.org and add an SSH public key to it (a key only for this).
The package names `gitkraft` and `gitkraft-bin` are free; the first push creates them. Then:

    gh secret set AUR_USERNAME -R sorinirimies/gitkraft
    gh secret set AUR_EMAIL -R sorinirimies/gitkraft
    gh secret set AUR_SSH_PRIVATE_KEY -R sorinirimies/gitkraft < ~/.ssh/aur

**winget.** Fork `microsoft/winget-pkgs` to your account. Submit the first version by hand with
[komac](https://github.com/russellbanks/Komac) after a release (it reads the installer, an NSIS
per-machine install with `/S` for silent):

    komac new SorinIrimies.GitKraft --version X.Y.Z \
      --urls https://github.com/sorinirimies/gitkraft/releases/download/vX.Y.Z/gitkraft-X.Y.Z-windows-x86_64-setup.exe \
      --submit

Once Microsoft merges it, create a classic token with `public_repo` and save it as `WINGET_TOKEN`;
later versions are opened as pull requests by the `winget` job.

**Flathub.** See `flatpak/SUBMISSION.md` (fork `flathub/flathub`, PR against `new-pr`, then
`FLATHUB_TOKEN`). Build it once on Linux before the PR; it has never been built.

## Known limits

- **macOS Gatekeeper.** The app is signed ad hoc, not notarized (by choice: no paid Apple
  account, no Mac App Store), so a downloaded copy is refused until
  `xattr -dr com.apple.quarantine /Applications/GitKraft.app`; the cask says so.
- **Microsoft Store.** It takes Win32 installers, but they must be signed with a certificate from a
  trusted CA, and our installer is unsigned. winget and Scoop do not need that; Windows SmartScreen
  may still warn on first run.
- **Homebrew on Linux** uses the glibc-linked tarballs built on `ubuntu-latest`, so it needs a
  distro with that glibc or newer.
- **Homebrew trust.** Recent Homebrew versions require third-party taps to be trusted once:
  `brew trust --tap sorinirimies/tap`.
