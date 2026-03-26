--------------------------------------------------------------------------------
-- Status Bar and Tab Bar Setup
--------------------------------------------------------------------------------

-- Bar
srwm.bar.padding_horizontal(0)
srwm.bar.padding_vertical(15)

-- Tab
srwm.bar.tab_height(35)
srwm.bar.tab_tile_vertical_padding(15)
srwm.bar.tab_tile_inner_padding_horizontal(15)
srwm.bar.tab_tile_outer_padding_horizontal(15)

-- Looks
srwm.bar.show(true)
srwm.bar.top(false)
srwm.bar.tab_top(true)

-- Set fonts
srwm.bar.fonts("Terminess Nerd Font:size=16")

-- Workspace underline
srwm.bar.ws_underline_padding(5)
srwm.bar.ws_underline_size(2)
srwm.bar.ws_underline_offset(0)
srwm.bar.ws_underline_all(false)

-- Systray
srwm.bar.systray(true)
srwm.bar.systray_spacing(2)
srwm.bar.systray_pinning(0) -- value is monitors order, 0 is first monitor

-- Set polling interval (seconds)
srwm.bar.interval(1)
