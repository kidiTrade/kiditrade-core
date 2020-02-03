with import <nixpkgs> { };
let
  moz_overlay = import (builtins.fetchTarball
    "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz");
  native_pkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  rust = (native_pkgs.rustChannels.stable.rust.override {
    extensions = [
      "rls-preview"
      "rust-analysis"
      "rust-src"
      "rustfmt-preview"
      "clippy-preview"
    ];
  });
in pkgs.mkShell { buildInputs = [ diesel-cli rust ]; }
