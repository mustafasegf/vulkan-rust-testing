{
  description = "A devShell example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          rust-overlay.overlays.default
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        optional = {
          darwin = pkgs.lib.optionals pkgs.stdenv.isDarwin;
          linux = pkgs.lib.optionals pkgs.stdenv.isLinux;
        };
        defaultPackage =
          with pkgs;
          rustPlatform.buildRustPackage {
            pname = "enabler";
            version = "0.0.0";
            cargoLock.lockFile = ./Cargo.lock;
            src = lib.cleanSource ./.;

            # LIBTOOL = "${lib.getExe glibtool}";
            ## Compiler or Toolchain flags
            # CXXFLAGS = "-Wno-unused-variable -Wno-unused-parameter -Wunused-parameter";
            # CFLAGS = "-Wno-unused-variable -Wunused-parameter";
            # cmakeFlags = [
            #   "-Wno-dev"
            #   "-DCMAKE_BUILD_TYPE=Release"
            # ];
            cmakeFlags = optional.darwin [
              "-DCMAKE_AR=${lib.getExe glibtool}"
            ];

            nativeBuildInputs =
              [
                shaderc
                clang
                cmake
                git
                python3
                glibtool
                pkg-config
                vulkan-loader
                vulkan-tools
                vulkan-headers
              ]
              ++ optional.darwin [
                glibtool
                automake
                autoconf
              ];

            buildInputs =
              [
                gcc
                cmake
                shaderc
                vulkan-loader
              ]
              ++ optional.darwin [
                glibtool
                automake
                autoconf
                darwin.moltenvk
              ];

            LD_LIBRARY_PATH = lib.makeLibraryPath (
              [
                shaderc
                vulkan-loader
              ]
              ++ optional.linux [
                xorg.libX11
                xorg.libXcursor
                xorg.libXrandr
                xorg.libXi
              ]
            );
          };
      in
      {
        packages.default = defaultPackage;
        devShells.default =
          with pkgs;
          mkShell {
            inputsFrom = [ defaultPackage ];
            shellHook = optional.darwin ''
              export DYLD_LIBRARY_PATH=${vulkan-loader}/lib:$DYLD_LIBRARY_PATH
            '';
          };
      }
    );
}
