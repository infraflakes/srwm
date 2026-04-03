# elementary-pastel icon theme

Custom icon theme: Mignon-pastel app icons + elementary for everything else.

## How it works

`elementary-pastel` at `~/.local/share/icons/elementary-pastel/` contains only `scalable/apps/` — ~5k symlinks pointing to Mignon-pastel SVGs. The `index.theme` sets `Inherits=elementary,Adwaita,hicolor`, so any non-app icon (status, actions, devices, mimes, places, etc.) falls through to elementary.

Lookup order: elementary-pastel (apps only) → elementary (everything else) → Adwaita → hicolor.

## Setup

1. Build and install [elementary icons](https://github.com/elementary/icons) → ends up at `~/.local/share/icons/elementary/`
2. Install [Mignon-pastel](https://github.com/igorfmoraes/Mignon-icon-theme) → ends up at `~/.local/share/icons/Mignon-pastel/`
3. Create `~/.local/share/icons/elementary-pastel/` with an `index.theme` inheriting elementary, and symlink `scalable/apps/` icons from Mignon-pastel
