# macOS packaging

`nu scripts/ci/package_desktop.nu macos <version>` builds `dist/gitkraft-<version>-macos.dmg`:

- `GitKraft.app` — the desktop GUI (`gitkraft`), with the bundle id from `APP` in
  `scripts/project.nu`, signed ad hoc (Apple silicon refuses unsigned code), with the icon
  `packaging/macos/gitkraft.icns` (16–1024 px on Apple's icon grid, made by `just icons`
  from `assets/icons/gitkraft.svg`)
- `Command Line Tools/` — `gitkraft-tui`

It merges the `aarch64-apple-darwin` and `x86_64-apple-darwin` builds into universal binaries
with `lipo`.

```bash
just package-macos 1.2.15   # builds both Mac targets, then the DMG
```
