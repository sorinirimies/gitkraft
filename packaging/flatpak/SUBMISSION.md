# Flathub submission

Everything for the first submission to `flathub/flathub`. Do it once; after that
`scripts/ci/update_flathub.nu` (the release workflow) keeps the app repo current.

## Order

1. Cut the release: `just bump 1.6.0`. The tag must exist, because the manifest and the
   metainfo screenshots point at `v1.6.0`.
2. Fork `flathub/flathub`, branch off `new-pr`, add a folder named after the app id with:
   - `io.github.sorinirimies.GitKraft.yml` (this directory's manifest, with the real commit:
     `git rev-parse v1.6.0^{commit}`)
   - `cargo-sources.json` (`python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json`,
     from `flatpak-builder-tools/cargo`)
3. Open the PR against `new-pr` with the text below.
4. After it is accepted, Flathub creates `flathub/io.github.sorinirimies.GitKraft`; give the
   release workflow push access via the `FLATHUB_TOKEN` secret.

## PR text

> Add io.github.sorinirimies.GitKraft
>
> GitKraft is a Git client written in Rust (iced). Besides the usual history, staging and diff
> views it talks to GitHub, GitLab, Gitea/Forgejo and Codeberg: pull requests, issues,
> notifications and CI runs in one window.
>
> A few things you may want to ask about:
>
> - `--filesystem=home`: it opens repositories wherever the user keeps them, and reads
>   `~/.ssh/config` to find the right key for a remote. I could not find a narrower
>   permission that works for arbitrary repository locations.
> - `--socket=ssh-auth`: so pushes and fetches over SSH use the user's agent.
> - `--share=network`: Git remotes and the forge APIs.
> - `--talk-name=org.kde.StatusNotifierWatcher`: the optional tray icon.
> - `--talk-name=org.freedesktop.Notifications`: desktop alerts for new notifications.
>   Start at login goes through the Background portal.
>
> I am the upstream author. It builds offline from `cargo-sources.json`.

## Not verified here

Built on macOS, where `flatpak-builder` does not run. The manifest, the metainfo
(`appstreamcli validate --pedantic`) and the generated `cargo-sources.json` (1671 crates, no git
dependencies) were checked, but the Flatpak itself has not been built. Build it once on Linux
before opening the PR:

    flatpak-builder --user --install --force-clean build packaging/flatpak/io.github.sorinirimies.GitKraft.yml
