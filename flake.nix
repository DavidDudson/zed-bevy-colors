{
  description = "zed-bevy-color dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rust
            cargo-criterion
            cargo-watch
            cargo-deny
            just
            lefthook
            release-plz
            git-cliff
            typos
            taplo
          ];

          RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";

          shellHook = ''
            if [ -d .git ] && [ ! -f .git/hooks/pre-commit ]; then
              ${pkgs.lefthook}/bin/lefthook install >/dev/null
            fi
          '';
        };
      });
}
