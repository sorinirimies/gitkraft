# GitKraft for Nix: the GUI, TUI, web and MCP binaries from one workspace build.
# The flake at the repository root builds it from the checkout; a nixpkgs copy
# would swap `src` for fetchFromGitHub and `cargoLock` for `cargoHash`.
{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  perl,
  openssl,
  zlib,
  fontconfig,
  libxkbcommon,
  wayland,
  vulkan-loader,
  libGL,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
  aws-lc,
  src,
  version,
}:

let
  # dlopen()ed at run time by the GUI (winit, wgpu), so they go on its RUNPATH.
  runtimeLibs = [
    wayland
    libxkbcommon
    vulkan-loader
    libGL
    libx11
    libxcursor
    libxi
    libxrandr
  ];
in
rustPlatform.buildRustPackage {
  pname = "gitkraft";
  inherit version src;

  cargoLock.lockFile = "${src}/Cargo.lock";

  cargoBuildFlags = [
    "-p" "gitkraft"
    "-p" "gitkraft-tui"
    "-p" "gitkraft-web"
    "-p" "gitkraft-mcp"
  ];

  nativeBuildInputs = [ pkg-config perl ];
  # aws-lc: nixpkgs builds aws-lc-sys (rustls) against the packaged library.
  buildInputs = [ openssl zlib fontconfig aws-lc ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ libxkbcommon wayland libx11 ];

  # Link the system OpenSSL instead of building the vendored copy.
  env.OPENSSL_NO_VENDOR = "1";

  # The test suite runs in CI; it needs a git identity and writable homes.
  doCheck = false;

  postInstall = ''
    install -Dm644 packaging/linux/io.github.sorinirimies.GitKraft.desktop -t $out/share/applications
    install -Dm644 packaging/linux/io.github.sorinirimies.GitKraft.metainfo.xml -t $out/share/metainfo
    install -Dm644 assets/icons/gitkraft.svg $out/share/icons/hicolor/scalable/apps/io.github.sorinirimies.GitKraft.svg
    for n in 16 22 24 32 48 64 96 128 256 512; do
      install -Dm644 assets/icons/gitkraft-$n.png $out/share/icons/hicolor/''${n}x''${n}/apps/io.github.sorinirimies.GitKraft.png
    done
  '';

  postFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    patchelf --add-rpath ${lib.makeLibraryPath runtimeLibs} $out/bin/gitkraft
  '';

  meta = {
    description = "Git IDE written entirely in Rust: desktop GUI and terminal UI";
    homepage = "https://github.com/sorinirimies/gitkraft";
    license = lib.licenses.mit;
    mainProgram = "gitkraft";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
