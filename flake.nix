{
  description = "Polars network plugin.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        overrides = builtins.fromTOML (builtins.readFile (self + "/rust-toolchain.toml"));
      in {
        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [pkg-config maturin python3];
          buildInputs = with pkgs; [
            clang
            llvmPackages.bintools
            rustup
            mold
            sccache
          ];

          CC = "${pkgs.clang}/bin/clang";
          CXX = "${pkgs.clang}/bin/clang++";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = CC;
          RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

          rustLibPaths = [
            # add libraries here (e.g. pkgs.libvmi)
          ];

          RUSTC_VERSION = overrides.toolchain.channel;

          LIBCLANG_PATH = pkgs.lib.makeLibraryPath [pkgs.llvmPackages_latest.libclang.lib];

          shellHook = ''
            export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
            export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
          '';

          RUSTFLAGS = builtins.concatStringsSep " " (
            ["-C link-arg=-fuse-ld=mold"]
            ++ (builtins.map (a: "-L ${a}/lib") rustLibPaths)
          );

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);

          BINDGEN_EXTRA_CLANG_ARGS =
            # Includes normal include path
            (builtins.map (a: ''-I"${a}/include"'') [
              # add dev libraries here (e.g. pkgs.libvmi.dev)
              pkgs.glibc.dev
            ])
            # Includes with special directory paths
            ++ [
              ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
              ''-I"${pkgs.glib.dev}/include/glib-2.0"''
              ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
            ];
        };
      }
    );
}
