{
  description = "srwc — a trackpad-first infinite canvas Wayland compositor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    dagger = {
      url = "github:dagger/nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    dagger,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    nativeBuildInputs = with pkgs; [
      pkg-config
      pkgs.makeWrapper
      rustPlatform.bindgenHook
    ];

    buildInputs = with pkgs; [
      adwaita-icon-theme
      wayland
      wayland-protocols
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
    packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
      pname = "srwc";
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

      src = pkgs.lib.cleanSourceWith {
        src = ./.;
        filter = path: type: let
          baseName = builtins.baseNameOf path;
        in
          baseName != "target" && baseName != ".git" && baseName != ".direnv";
      };

      cargoLock = {
        lockFile = ./Cargo.lock;
        allowBuiltinFetchGit = true;
      };

      inherit nativeBuildInputs buildInputs;

      env = {
        RUSTFLAGS = toString (
          map (arg: "-C link-arg=" + arg) [
            "-Wl,--push-state,--no-as-needed"
            "-lEGL"
            "-lwayland-client"
            "-Wl,--pop-state"
          ]
        );
      };

      postFixup = ''
        patchelf --add-rpath "${pkgs.lib.makeLibraryPath runtimeLibs}" $out/bin/srwc
        wrapProgram $out/bin/srwc \
          --prefix XCURSOR_PATH : "${pkgs.adwaita-icon-theme}/share/icons" \
          --prefix PATH : "${pkgs.lib.makeBinPath [pkgs.xdg-utils pkgs.libnotify]}"
      '';

      postInstall = ''
        install -Dm644 resources/srwc.desktop $out/share/wayland-sessions/srwc.desktop
        install -Dm644 resources/srwc-portals.conf $out/share/xdg-desktop-portal/srwc-portals.conf
        install -Dm644 resources/config.example.toml $out/etc/srwc/config.toml
        for f in resources/extras/wallpapers/*.glsl; do
          install -Dm644 "$f" "$out/share/srwc/wallpapers/$(basename "$f")"
        done
      '';

      passthru.providedSessions = ["srwc"];

      meta = with pkgs.lib; {
        description = "A trackpad-first infinite canvas Wayland compositor";
        license = licenses.gpl3Plus;
        platforms = ["x86_64-linux"];
        mainProgram = "srwc";
      };
    };

    devShells.${system}.default = pkgs.mkShell {
      buildInputs =
        buildInputs
        ++ (with pkgs; [
          cargo
          clippy
          rustfmt
          cargo-edit
          rustc
          dagger.packages.${stdenv.hostPlatform.system}.dagger
        ]);

      inherit runtimeLibs;
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
    };
  };
}
