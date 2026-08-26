{
  description = "Weylus Community Edition";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      rust = pkgs.rust-bin.stable.latest.default;
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "weylus-community-edition";
        version = "unstable";
        src = ./.;

        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = {
            "autopilot-0.4.0" = "sha256-UvniDuAntd444Qim3xdy7KRiKpWDtlVRJeXs4BrzTUQ=";
          };
        };

        nativeBuildInputs = with pkgs; [
          rust
					makeWrapper
          pkg-config
          nodejs
          typescript
          gnumake
          bash
          nasm
          cmake
          clang
        ];

        buildInputs = with pkgs; [
          wayland
          pipewire
          ffmpeg
          x264
          libva
          libdrm
          libX11
          libxcb
          libxkbcommon
          libXext
          libXrandr
          libXfixes
          libXcomposite
          libXi
          libXcursor
          libXinerama
          libXtst
          libXv
          xcbutil
          xcbutilimage
          xcbutilkeysyms
          xcbutilrenderutil
          dbus
          pango
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          xdg-desktop-portal
        ];

        cargoBuildFlags = [
          "--features"
          "ffmpeg-system"
        ];

        doCheck = false;

        postInstall = let
          gstPlugins = pkgs.lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" [
            pkgs.gst_all_1.gst-plugins-base
            pkgs.pipewire
          ];
        in ''
          wrapProgram $out/bin/weylus \
            --set GST_PLUGIN_PATH_1_0 "${gstPlugins}" \
            --set GST_PLUGIN_SYSTEM_PATH_1_0 ""
        '';
      };

      apps.default = flake-utils.lib.mkApp {
        drv = self.packages.${system}.default;
      };

      devShells.default = pkgs.mkShell {
        packages = [
          rust
          pkgs.pkg-config
          pkgs.nodejs
          pkgs.typescript
          pkgs.gnumake
          pkgs.bash
          pkgs.nasm
          pkgs.clang
          pkgs.ffmpeg
          pkgs.x264
          pkgs.libva
          pkgs.libdrm
          pkgs.libX11
          pkgs.libxcb
          pkgs.libxkbcommon
          pkgs.libXext
          pkgs.libXrandr
          pkgs.libXfixes
          pkgs.libXcomposite
          pkgs.libXi
          pkgs.libXcursor
          pkgs.libXinerama
          pkgs.libXtst
          pkgs.libXv
          pkgs.xcbutil
          pkgs.xcbutilimage
          pkgs.xcbutilkeysyms
          pkgs.xcbutilrenderutil
          pkgs.dbus
          pkgs.pango
          pkgs.gstreamer
          pkgs.gst_all_1.gstreamer
          pkgs.gst_all_1.gst-plugins-base
          pkgs.pipewire
          pkgs.xdg-desktop-portal
        ];
      };
    });
}
