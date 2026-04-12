<h1 align="center">srwc (Serein Wayland Compositor)</h1>
<p align="center">A trackpad-first infinite canvas Wayland compositor.</p>
<p align="center">
    <a href="LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
    <a href="https://github.com/infraflakes/srwc/releases"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/infraflakes/srwc?logo=github"></a>
</p>

Traditional window managers arrange windows to fit your screen, but on `srwc` windows float on an infinite 2D canvas and you move the viewport around them.

> **WARNING:** Project is in early development, please leave suggestions and report bugs if you find one.

## Lineage

srwc is a fork of [driftwm](https://github.com/malbiruk/driftwm), which introduced optimizations, text-input-v3 protocol, screen casting via GNOME portal, built-in screenshot utility, ext_data_control_v1 protocol, and more.

## Installation

### NixOS

Add the `srwc` flake:

```nix
inputs = {
  nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  srwc = {
      url = "github:infraflakes/srwc";
      inputs.nixpkgs.follows = "nixpkgs";
    };
};
```

In your `configuration.nix`:

```nix
{ inputs, ... }: {
  imports = [
    inputs.srwc.nixosModules.default
  ];

  programs.srwc.enable = true;
}
```

### CLI install

srwc embeds all required session artifacts in the binary:

```bash
srwc install
```

This installs config, portal configuration, and wallpaper shaders to user directories, then optionally installs the `.desktop` session file (requires sudo).

To remove:

```bash
srwc uninstall
```

### Running

```bash
srwc start                    # auto-detect backend
srwc start --backend winit    # nested (inside existing session)
srwc start --backend udev     # bare metal (from TTY)
```

Running `srwc` without arguments shows the command list. For display manager integration, select "srwc" from the session menu.

### Other commands

```bash
srwc check-config    # validate config and exit
srwc --version       # print version
```

## Features

### Pan & zoom

Infinite 2D canvas with viewport panning, zoom, and scroll momentum. A quick flick carries the viewport smoothly until friction stops it.

| Input              | Action            | Context   |
| ------------------ | ----------------- | --------- |
| 3-finger swipe     | Pan viewport      | anywhere  |
| Trackpad scroll    | Pan viewport      | on-canvas |
| `Mod` + LMB drag   | Pan viewport      | anywhere  |
| `Mod+Ctrl` + arrow | Pan viewport      | —         |
| 2-finger pinch     | Zoom              | on-canvas |
| 3-finger pinch     | Zoom              | anywhere  |
| `Mod` + scroll     | Zoom at cursor    | anywhere  |
| `Mod+=` / `Mod+-`  | Zoom in / out     | —         |
| `Mod+0` / `Mod+Z`  | Reset zoom to 1.0 | —         |

### Window navigation

Jump to the nearest window in any direction via cone search. MRU cycling (`Alt-Tab`) with hold-to-commit. Zoom-to-fit shows all windows at once.

| Input                        | Action                                     |
| ---------------------------- | ------------------------------------------ |
| 4-finger swipe               | Jump to nearest window (natural direction) |
| `Mod` + arrow                | Jump to nearest window in direction        |
| `Alt-Tab` / `Alt-Shift-Tab`  | Cycle windows (MRU)                        |
| 4-finger pinch in / `Mod+W`  | Zoom-to-fit (overview)                     |
| 4-finger hold / `Mod+C`      | Center focused window                      |

### Move, resize, maximize

Move windows by doubletap-swiping on them. Resize with `Alt` + 3-finger swipe. Windows snap to nearby edges magnetically during drag. Drag to the viewport edge and the canvas auto-pans.

Fit-window (`Mod+M`) centers the viewport, resets zoom to 1.0, and resizes the window to fill the screen. Fullscreen (`Mod+F`) is a viewport mode — any canvas action naturally exits it.

| Input                         | Action                        |
| ----------------------------- | ----------------------------- |
| 3-finger doubletap-swipe      | Move window                   |
| `Alt` + LMB drag              | Move window                   |
| `Alt` + 3-finger swipe        | Resize window                 |
| `Alt` + RMB drag              | Resize window                 |
| `Alt` + MMB click / `Mod+M`   | Fit window (maximize/restore) |
| `Mod` + MMB click / `Mod+F`   | Toggle fullscreen             |
| `Mod+Shift` + arrow           | Nudge window 20px             |


### Screenshots

Built-in interactive screenshot UI. Drag to select a region, press Space/Enter to save, Ctrl+C for clipboard only, Escape to cancel, P to toggle cursor visibility.

| Input         | Action                          |
| ------------- | ------------------------------- |
| `Print`       | Open interactive screenshot UI  |
| `Ctrl+Print`  | Instant full-screen screenshot  |

### Screencasting

Native GNOME portal screencasting via PipeWire. OBS, Firefox, Discord, and other apps can capture both monitors and individual windows through the standard `xdg-desktop-portal-gnome` flow — no `xdg-desktop-portal-wlr` needed.

Requires: `xdg-desktop-portal`, `xdg-desktop-portal-gnome`, `xdg-desktop-portal-gtk`, `pipewire`.

### Infinite background

The background is part of the canvas — it scrolls and zooms with the viewport. Uses **GLSL shaders** (default: dot grid).

```toml
[background]
shader_path = "~/.config/srwc/bg.glsl"
```

### Window rules

Match windows by `app_id` and/or `title` (glob patterns). Control position, size, decoration mode, blur, opacity, and widget behavior.

**Widgets**: `widget = true` pins a window in place — immovable, below normal windows, excluded from Alt-Tab.

```toml
[[window_rules]]
app_id = "Alacritty"
opacity = 0.85
blur = true

[[window_rules]]
app_id = "my-clock"
position = [50, 50]
widget = true
decoration = "none"
```

### Multi-monitor

Multiple monitors are independent viewports on the same canvas — different zoom levels, overlapping views. An outline on each monitor shows where the other monitors' viewports are.

## Configuration

Config file: `~/.config/srwc/config.toml` (respects `XDG_CONFIG_HOME`).

See [`config.example.toml`](./resources/config.example.toml) for all options.

## Acknowledgments

srwc would not exist without these projects:

- **[driftwm](https://github.com/malbiruk/driftwm)**.
- **[niri](https://github.com/YaLTeR/niri)**.
- **[Smithay](https://github.com/Smithay/smithay)**.

## License

GPL-3.0-or-later
