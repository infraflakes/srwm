{
  description = "srwc — a trackpad-first infinite canvas Wayland compositor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    dagger = {
      url = "github:dagger/nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-parts,
    dagger,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux"];

      perSystem = {
        config,
        pkgs,
        system,
        ...
      }: let
        runtimeLibs = with pkgs; [
          wayland
          seatd
          libdisplay-info
          libinput
          libgbm
          libxkbcommon
          libdrm
          libglvnd
          libx11
          libxcursor
          libxrandr
          libxi
          libxcb
          pixman
          dbus
          pipewire
          systemd
        ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "srwc";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type: let
              baseName = builtins.baseNameOf path;
            in
              ! (builtins.elem baseName ["target" ".git" ".direnv"]);
          };

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
            rustPlatform.bindgenHook
          ];

          buildInputs = runtimeLibs ++ [pkgs.adwaita-icon-theme pkgs.wayland-protocols];

          postFixup = ''
            patchelf --add-rpath "${pkgs.lib.makeLibraryPath runtimeLibs}" $out/bin/srwc
            wrapProgram $out/bin/srwc \
              --prefix XCURSOR_PATH : "${pkgs.adwaita-icon-theme}/share/icons" \
              --prefix PATH : "${pkgs.lib.makeBinPath [pkgs.xdg-utils pkgs.libnotify pkgs.xwayland-satellite]}"
          '';

          postInstall = ''
            install -Dm644 resources/srwc.desktop $out/share/wayland-sessions/srwc.desktop
            install -Dm644 resources/srwc-portals.conf $out/share/xdg-desktop-portal/srwc-portals.conf
            install -Dm644 resources/config.example.toml $out/etc/srwc/config.toml
            mkdir -p $out/etc/srwc/wallpapers
            cp resources/extras/wallpapers/*.glsl $out/etc/srwc/wallpapers/ || true
          '';

          passthru.providedSessions = ["srwc"];
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [config.packages.default];
          buildInputs = with pkgs; [
            cargo
            clippy
            rustfmt
            cargo-edit
            rustc
            seatd.dev
            udev.dev
            go_1_26
            golangci-lint
            dagger.packages.${stdenv.hostPlatform.system}.dagger
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

          shellHook = ''
            go env -w GOPATH=$HOME/.local/share/go
          '';
        };
      };

      flake = {
        nixosModules.default = {
          config,
          lib,
          pkgs,
          ...
        }: {
          options.programs.srwc.enable = lib.mkEnableOption "srwc compositor";
          config = lib.mkIf config.programs.srwc.enable {
            environment.systemPackages = [self.packages.${pkgs.system}.default];
            environment.etc."srwc".source = "${self.packages.${pkgs.system}.default}/etc/srwc";
            services.displayManager.sessionPackages = [self.packages.${pkgs.system}.default];

            # Link portals config so screen sharing works
            xdg.portal.configPackages = [self.packages.${pkgs.system}.default];
          };
        };
      };
    };
}
