{
  description = "Build environment for srwm (Go + C core)";
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
        version = "0.3.0";
        pkgs = import nixpkgs {inherit system;};
        # Use pkgsStatic to get X11 libs compiled against musl
        staticPkgs = pkgs.pkgsStatic;
        staticImlib2 = staticPkgs.imlib2.overrideAttrs (oldAttrs: {
          configureFlags =
            (oldAttrs.configureFlags or [])
            ++ [
              "--with-x"
              "--x-includes=${staticPkgs.libx11.dev}/include"
              "--x-libraries=${staticPkgs.libx11.out}/lib"
            ];
          # Ensure X11 dependencies are explicitly in the build path for imlib2
          buildInputs =
            oldAttrs.buildInputs
            ++ [
              staticPkgs.libx11
              staticPkgs.libxext
              staticPkgs.xorgproto
            ];
        });

        nativeBuildInputs = with pkgs; [
          go_1_26
          golangci-lint
          zig
          pkg-config
          upx
        ];

        buildInputs = with staticPkgs; [
          libx11
          libxinerama
          libxft
          libxrender
          libxext
          libxcb
          xorgproto
          staticImlib2
          freetype
          fontconfig
        ];
      in {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
          hardeningDisable = ["fortify"];

          shellHook = ''
            go env -w GOPATH=$HOME/.local/share/go
            export CGO_ENABLED=1
            export ZIG_GLOBAL_CACHE_DIR="/tmp"
            export CC="zig cc -target x86_64-linux-musl"
            export CGO_CFLAGS="$(pkg-config --cflags x11 xinerama xft xrender imlib2)"
            export CGO_LDFLAGS="$(pkg-config --libs --static x11 x11-xcb xcb-shm xinerama xft xrender imlib2)"
            echo "srwm dev shell ready. Run 'make build' to build."
          '';
        };

        packages.default = (pkgs.buildGoModule.override {go = pkgs.go_1_26;}) {
          pname = "srwm";
          version = "${version}";
          hardeningDisable = ["fortify"]; # This tells Nix not to inject -D_FORTIFY_SOURCE=2 into the compiler flags, so fprintf stays as fprintf and links correctly against musl
          src = ./.;
          vendorHash = "sha256-cy/ECWJDjlUnIpPNYRWGMkyP+WV1JxzUHrV0pzGoF1o=";
          env = {
            CGO_ENABLED = "1";
            CC = "zig cc -target x86_64-linux-musl";
            ZIG_GLOBAL_CACHE_DIR = "/tmp";
          };

          inherit nativeBuildInputs buildInputs;

          preBuild = ''
            export CGO_CFLAGS="$(pkg-config --cflags x11 xinerama xft xrender imlib2)"
            export CGO_LDFLAGS="$(pkg-config --libs --static x11 x11-xcb xcb-shm xinerama xft xrender imlib2)"
          '';

          ldflags = [
            "-w"
            "-s"
            "-linkmode external"
            "-extld 'zig cc -target x86_64-linux-musl'"
            "-extldflags '-static'"
            "-X github.com/infraflakes/srwm/cmd.Version=${version}"
          ];
          tags = ["netgo"];
          postInstall = ''
            upx --best --lzma $out/bin/srwm
          '';
        };
      }
    );
}
