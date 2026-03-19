# Serein Window Manager
A fully statically linked X11 floating, zooming window manager. Built with Go, C and Lua.

![Preview](./assets/screenshot_default.png)

## Installation

### Download Binary (Recommended)
Since `srwm` is built as a zero-dependency static binary, you can run it on almost any Linux distribution.

1. Download the latest release from the [Releases](https://github.com/infraflakes/srwm/releases) page.
2. Make it executable and move it to your path:
   ```bash
   chmod +x srwm-v*-linux-amd64
   sudo mv srwm-v*-linux-amd64 /usr/local/bin/srwm
   ```

### Getting started

> [!CAUTION]
> You should modify the font config at ~/.config/srwm/bar.lua
> And keybindings at ~/.config/srwm/keybindings.lua before starting

You can generate the default config with `srwm kickstart`:

```bash
srwm kickstart
```

Then start the window manager with `srwm start` (srwm already has X Server process handler built in):
```bash
srwm start
```

If you use the default bar's widgets config, the bar's widgets depend on some dependencies:

- iw
- xset
- bc

### Building from scratch

Current the project only supports building with flakes (fully statically linked).
After having all the dependencies you can easily build the window manager:

```bash
nix build .#default
```

Or

```bash
make build
```

## Acknowledgements

Special thanks to:

- The developers who worked on dwm to make it possible.

- [Siduck](https://github.com/siduck) and other contributors for chadwm which `srwm` originally based on.

- [wh1tepearl](https://codeberg.org/wh1tepearl) for the canvas layout `srwm` based on.
---
