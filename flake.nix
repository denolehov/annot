{
  description = "annot dev shell — Tauri v2 + SvelteKit on Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Runtime env vars needed by the binary (same as devShell)
        gstPluginPath = pkgs.lib.concatStringsSep ":" [
          "${pkgs.gst_all_1.gstreamer.out}/lib/gstreamer-1.0"
          "${pkgs.gst_all_1.gst-plugins-base}/lib/gstreamer-1.0"
          "${pkgs.gst_all_1.gst-plugins-good}/lib/gstreamer-1.0"
          "${pkgs.gst_all_1.gst-plugins-bad}/lib/gstreamer-1.0"
        ];
      in {
        # Install with: nix profile install path:.
        # Requires the binary to be pre-built: pnpm tauri build
        packages.default = pkgs.runCommand "annot" {
          nativeBuildInputs = [ pkgs.makeWrapper ];
          meta.mainProgram = "annot";
        } ''
          mkdir -p $out/bin
          makeWrapper ${./src-tauri/target/release/annot} $out/bin/annot \
            --set GDK_BACKEND x11 \
            --set WEBKIT_DISABLE_DMABUF_RENDERER 1 \
            --set GST_PLUGIN_SYSTEM_PATH "${gstPluginPath}" \
            --set GIO_MODULE_DIR "${pkgs.glib-networking}/lib/gio/modules"
        '';

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            cargo
            rustc
            rustfmt
            clippy
            nodejs_24
            pnpm
          ];

          buildInputs = with pkgs; [
            # Tauri v2 requires WebKit2GTK 4.1 (not 4.0)
            webkitgtk_4_1
            gtk3
            glib
            glib-networking   # TLS support for WebKit
            libsoup_3
            openssl
            cairo
            pango
            gdk-pixbuf
            librsvg
            at-spi2-atk
            at-spi2-core

            # GStreamer — required by WebKit2GTK for media/video support
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base   # provides appsink
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad

            # arboard (clipboard) — X11 backend
            libxcb
            xclip

            # tauri-plugin-opener uses xdg-open on Linux
            xdg-utils
          ];

          # PKG_CONFIG_PATH is usually set automatically by mkShell for buildInputs,
          # but openssl sometimes needs a nudge.
          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.webkitgtk_4_1.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            # GStreamer plugin path so WebKit2GTK can find appsink and friends
            export GST_PLUGIN_SYSTEM_PATH="${pkgs.gst_all_1.gstreamer.out}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-base}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-good}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-bad}/lib/gstreamer-1.0"

            # Force XWayland — WebKit2GTK's native Wayland/DMABuf renderer is broken on NixOS
            export GDK_BACKEND=x11
            export WEBKIT_DISABLE_DMABUF_RENDERER=1

            # GIO modules path so glib-networking (TLS) is found by WebKit at runtime
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"

            # pnpm@10.28.0 is pinned in package.json; nixpkgs ships the same version.
            # Disable strict corepack enforcement so the nixpkgs pnpm binary is used directly.
            export COREPACK_ENABLE_STRICT=0

            echo "annot dev shell ready"
            echo "  pnpm install     — install JS deps"
            echo "  pnpm tauri dev   — start dev server + Tauri window"
            echo "  pnpm tauri build — production build"
          '';
        };
      }
    );
}
