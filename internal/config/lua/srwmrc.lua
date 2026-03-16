--------------------------------------------------------------------------------
-- Configuration
--------------------------------------------------------------------------------
-- Note: These take effect on restart (srwm.restart())

-- Modular srwm configuration entry point
-- Each module is located in ~/.config/srwm/<name>.lua

include("general") -- Tags, gaps, border, snapping
include("bar") -- Bar settings
include("keybindings") -- Keyboard shortcuts
include("theming") -- Palette, colors, widgets, bar layout
include("startup") -- Autostart applications
include("canvas") -- Canvas layout
