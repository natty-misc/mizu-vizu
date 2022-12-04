let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
      rustfmt
      clippy
      pulseaudio
      llvmPackages_latest.llvm
      llvmPackages_latest.bintools
      llvmPackages_latest.lld
      rust-analyzer
    ];

    nativeBuildInputs = with pkgs; [
      libpulseaudio.dev
      cargo
      gcc
      pkg-config
    ];

    RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
      pkgs.libpulseaudio
    ]);

    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  }
