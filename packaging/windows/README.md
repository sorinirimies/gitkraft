# Windows Packaging

## NSIS Installer

The `installer.nsi` script builds a standard Windows installer (`.exe`) using
[NSIS](https://nsis.sourceforge.io/).

### Building locally

1. Install NSIS: https://nsis.sourceforge.io/Download
2. Build the binaries:
   ```
   cargo build --release -p gitkraft -p gitkraft-tui --target x86_64-pc-windows-msvc
   ```
3. Replace `@VERSION@` in the `.nsi` file with the actual version
4. Run: `makensis packaging\windows\installer.nsi`

The installer will be created at `dist\gitkraft-<version>-windows-x86_64-setup.exe`.

### Components

- **GitKraft GUI** (required) — the desktop GUI binary
- **GitKraft TUI** (optional) — the terminal UI binary
- **Add to PATH** (optional) — adds the install directory to `%PATH%`

The installer also creates:
- A Start Menu shortcut for the GUI
- A Desktop shortcut for the GUI
- Windows App Paths registry entries for both binaries
- A standard Add/Remove Programs entry

## Icon

The installer expects `packaging\windows\gitkraft.ico` to exist at build time.
See [`gitkraft.ico.txt`](gitkraft.ico.txt) for instructions on providing it.
