{
  description = "Build environment for swm (Go + C core)";
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
          zig
          pkg-config
        ];
        buildInputs = with staticPkgs; [
          libx11
          libxinerama
          libxft
          xorgproto
          staticImlib2
          freetype
          fontconfig
        ];
      in {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          # Automatically set the environment build command
          shellHook = ''
            export CGO_ENABLED=1
            export CC="zig cc -target x86_64-linux-musl"
            export CGO_CFLAGS="$(pkg-config --cflags x11 xinerama xft)"
            export CGO_LDFLAGS="$(pkg-config --libs --static x11 xinerama xft)"
            echo "Environment ready for swm build."
          '';
        };

        packages.default = pkgs.buildGoModule {
          pname = "swm";
          version = "0.1.0";
          src = ./.;
          vendorHash = null; # Set this if using modules

          inherit nativeBuildInputs buildInputs;

          CGO_ENABLED = 1;
          # Force Zig to handle the linking to avoid glibc leaks
          CC = "zig cc -target x86_64-linux-musl";

          ldflags = [
            "-w"
            "-s"
            "-linkmode external"
            "-extldflags '-static'"
          ];

          preBuild = ''
            export CGO_CFLAGS="$(pkg-config --cflags x11 xinerama xft)"
            export CGO_LDFLAGS="$(pkg-config --libs --static x11 xinerama xft)"
          '';
        };
      }
    );
}
