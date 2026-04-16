{ pkgs ? import <nixpkgs> { } }:

let
  runtimeLibraries = with pkgs; [
    fontconfig
    freetype
    libGL
    libxkbcommon
    vulkan-loader
    wayland
  ];
in
pkgs.mkShell {
  name = "paddler-second-brain-gui";

  nativeBuildInputs = with pkgs; [
    pkg-config
    weston
  ];

  buildInputs = runtimeLibraries ++ (with pkgs; [
    wayland-protocols
  ]);

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibraries}:''${LD_LIBRARY_PATH:-}"
    export XDG_RUNTIME_DIR="''${TMPDIR:-/tmp}/paddler-gui-runtime-$$"
    mkdir -p "$XDG_RUNTIME_DIR"
    chmod 700 "$XDG_RUNTIME_DIR"
  '';
}
