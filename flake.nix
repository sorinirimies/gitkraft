{
  description = "GitKraft: Git IDE written entirely in Rust (GUI, TUI, web, MCP)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
    in
    {
      packages = forAll (pkgs: rec {
        gitkraft = pkgs.callPackage ./packaging/nix/package.nix {
          src = self;
          inherit version;
        };
        default = gitkraft;
      });

      apps = forAll (pkgs:
        let
          pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.gitkraft;
          app = bin: { type = "app"; program = "${pkg}/bin/${bin}"; };
        in
        {
          default = app "gitkraft";
          gitkraft = app "gitkraft";
          gitkraft-tui = app "gitkraft-tui";
          gitkraft-web = app "gitkraft-web";
          gitkraft-mcp = app "gitkraft-mcp";
        });
    };
}
