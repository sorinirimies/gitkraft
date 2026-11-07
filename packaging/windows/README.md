# Windows packaging

`nu scripts/ci/package_desktop.nu windows <version>` writes an [NSIS](https://nsis.sourceforge.io/)
script for every binary in `scripts/project.nu` to `dist/installer.nsi` and compiles
`dist\gitkraft-<version>-windows-x86_64-setup.exe`: the GUI with Start-menu and desktop
shortcuts, the TUI, and the install folder on `PATH`.

Build the binaries first (`nu scripts/ci/build_release.nu x86_64-pc-windows-msvc`), with NSIS
installed (`nu scripts/ci/package_desktop.nu tools windows`). `packaging/windows/gitkraft.ico`
becomes the installer icon; `crates/gitkraft-gui/build.rs` embeds the same icon into
`gitkraft.exe`, which the shortcuts show. Both come from `assets/icons/gitkraft.svg` via
`just icons` (16–256 px, classic BMP entries: NSIS rejects PNG-compressed ones).
