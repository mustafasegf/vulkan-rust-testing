{
  description = "A devShell example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        devShells.default = mkShell {

          nativeBuildInputs = [ ];
          buildInputs = [
            cmake
            pkg-config
            libtool
            fontconfig

            vulkan-loader.out
            xorg.libX11.out
            xorg.libXcursor.out
            xorg.libXrandr.out
            xorg.libXi.out

            rust-bin.stable.latest.complete

          ];

          shellHook = ''
            LD_LIBRARY_PATH=$(nix eval --raw nixpkgs#vulkan-loader.out)/lib:$LD_LIBRARY_PATH
            LD_LIBRARY_PATH=$(nix eval --raw nixpkgs#xorg.libX11.out)/lib:$LD_LIBRARY_PATH
            LD_LIBRARY_PATH=$(nix eval --raw nixpkgs#xorg.libXcursor.out)/lib:$LD_LIBRARY_PATH
            LD_LIBRARY_PATH=$(nix eval --raw nixpkgs#xorg.libXrandr.out)/lib:$LD_LIBRARY_PATH
            LD_LIBRARY_PATH=$(nix eval --raw nixpkgs#xorg.libXi.out)/lib:$LD_LIBRARY_PATH
          '';

        };
      }
    );
}
