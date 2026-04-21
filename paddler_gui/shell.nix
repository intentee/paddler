{ pkgs ? import <nixpkgs> {} }:

let
  icedRuntimeLibs = with pkgs; [
    vulkan-loader
    libxkbcommon
    wayland
    libGL
    libx11
    libxcursor
    libxrandr
    libxi
    libxcb
  ];
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = icedRuntimeLibs ++ (with pkgs; [
    fontconfig
  ]);

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath icedRuntimeLibs;
}
