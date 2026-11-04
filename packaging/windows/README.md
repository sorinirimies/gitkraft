# Windows packaging

`nu scripts/ci/package_desktop.nu windows <version>` writes an [NSIS](https://nsis.sourceforge.io/)
script for every binary in `scripts/project.nu` to `dist/installer.nsi` and compiles
`dist\gitkraft-<version>-windows-x86_64-setup.exe`: the GUI with Start-menu and desktop
shortcuts, the TUI, and the install folder on `PATH`.

Build the binaries first (`nu scripts/ci/build_release.nu x86_64-pc-windows-msvc`), with NSIS
installed (`nu scripts/ci/package_desktop.nu tools windows`). A `packaging/windows/gitkraft.ico`
(classic BMP entries; NSIS rejects PNG-compressed ones) becomes the installer icon, e.g.:

    magick assets/icons/gitkraft-256.png -define icon:auto-resize=256,128,64,48,32,16 packaging/windows/gitkraft.ico
