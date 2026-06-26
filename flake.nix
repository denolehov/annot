{
  description = "annot — Human-in-the-loop annotation for AI workflows (Tauri v2 + SvelteKit)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        isDarwin = pkgs.stdenv.hostPlatform.isDarwin;
        isLinux = pkgs.stdenv.hostPlatform.isLinux;

        # GStreamer plugin path for WebKit2GTK media support (Linux only)
        gstPluginPath = pkgs.lib.optionalString isLinux (
          pkgs.lib.concatStringsSep ":" [
            "${pkgs.gst_all_1.gstreamer.out}/lib/gstreamer-1.0"
            "${pkgs.gst_all_1.gst-plugins-base}/lib/gstreamer-1.0"
            "${pkgs.gst_all_1.gst-plugins-good}/lib/gstreamer-1.0"
            "${pkgs.gst_all_1.gst-plugins-bad}/lib/gstreamer-1.0"
          ]
        );

        # Path suitable for XDG_DATA_DIRS so GTK can find gsettings schemas
        # (needed for Wayland display-scale reporting). Mirrors what
        # nixpkgs' gsettingsSchemaSetupHook sets up automatically as
        # GSETTINGS_SCHEMAS_PATH from buildInputs in the dev shell.
        gsettingsSchemasPath = pkgs.lib.optionalString isLinux (
          "${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}"
        );
      in
      {
        # Formatter (nixfmt)
        formatter = pkgs.nixfmt;

        # Binary package (requires pre-built artifacts from `pnpm tauri build`)
        # Install with: nix profile add path:. --impure
        # Remove with: nix profile remove annot
        packages.default =
          pkgs.runCommand "annot"
            {
              nativeBuildInputs = [ pkgs.makeWrapper ];
              meta.mainProgram = "annot";
            }
            (
              if isDarwin then
                ''
                  mkdir -p $out/bin
                  makeWrapper ${./src-tauri/target/release/annot} $out/bin/annot
                ''
              else
                # XDG_DATA_DIRS prefixed (not set) so any existing
                # desktop-environment entries are preserved. With this in
                # place, the installed binary runs on Wayland correctly;
                # no GDK_BACKEND override needed.
                ''
                  mkdir -p $out/bin
                  makeWrapper ${./src-tauri/target/release/annot} $out/bin/annot \
                    --prefix XDG_DATA_DIRS : "${gsettingsSchemasPath}" \
                    --set GST_PLUGIN_SYSTEM_PATH "${gstPluginPath}" \
                    --set GIO_MODULE_DIR "${pkgs.glib-networking}/lib/gio/modules"
                ''
            );

        # Development shell
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

          buildInputs =
            with pkgs;
            [ openssl ]
            ++ pkgs.lib.optionals isLinux [
              # Tauri v2 WebView (WebKit2GTK 4.1)
              webkitgtk_4_1
              gtk3
              glib
              glib-networking
              libsoup_3
              cairo
              pango
              gdk-pixbuf
              librsvg
              at-spi2-atk
              at-spi2-core

              # GStreamer (media/video support for WebKit)
              gst_all_1.gstreamer
              gst_all_1.gst-plugins-base
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-bad

              # Clipboard support (arboard X11 backend)
              libxcb
              xclip

              # tauri-plugin-opener
              xdg-utils
            ];

          shellHook = ''
            # OpenSSL pkg-config path
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            ${pkgs.lib.optionalString isLinux ''
              # WebKitGTK pkg-config
              export PKG_CONFIG_PATH="${pkgs.webkitgtk_4_1.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

              # GStreamer plugin discovery
              export GST_PLUGIN_SYSTEM_PATH="${gstPluginPath}"

              # Wayland needs gsettings schemas discoverable via XDG_DATA_DIRS
              # so GTK can report the correct display scale; otherwise the
              # WebView surface allocates at the wrong size.
              # (wiki.nixos.org/wiki/Tauri)
              export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"

              # GIO modules for TLS support
              export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
            ''}

            # Use nixpkgs pnpm directly (disable corepack strict mode)
            export COREPACK_ENABLE_STRICT=0

            echo "annot dev shell ready (${if isDarwin then "macOS" else "Linux"})"
            echo "  pnpm install     — install JS dependencies"
            echo "  pnpm tauri dev   — start dev server + Tauri window"
            echo "  pnpm tauri build — production build"
            echo "  pnpm demo        — open demo with test fixture"
            echo "  pnpm test        — run vitest suite"
            echo "  pnpm check       — svelte-check typecheck"
            echo "  pnpm run         — list all available scripts"
          '';
        };
      }
    );
}
