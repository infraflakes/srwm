-- Example configuration for srwm!

-- You can split your config to other <name>.lua files and include them as
-- include("<name>")
-- without .lua extension

--------------------------------------------------------------------------------
-- Configuration
--------------------------------------------------------------------------------
-- Note: These take effect on restart (srwm.restart())

-- Define workspace names (tags)
srwm.tags.set("1,2,3,4,5,6")

-- Appearance
srwm.cfg.borderpx(0)
srwm.cfg.gaps(0)
srwm.cfg.showbar(true)
srwm.cfg.topbar(true) -- need reimplementation
srwm.cfg.toptab(true) -- need reimplementation

-- Bar
srwm.cfg.bar_horizontal_padding(0)
srwm.cfg.bar_vertical_padding(15)

-- Tab
srwm.cfg.tab_vertical_padding(35) -- set to 0 breaks wm
srwm.cfg.tab_in_horizontal_padding(15)
srwm.cfg.tab_out_horizontal_padding(15)

-- Systray
srwm.cfg.systray_enable(true)
srwm.cfg.systray_spacing(2)
srwm.cfg.systray_pinning(0) -- value is monitors order, 0 is first monitor

-- Tag preview
srwm.cfg.tag_preview_size(4)
srwm.cfg.tag_preview_enable(false)

-- Tag underline
srwm.cfg.tag_underline_padding(5)
srwm.cfg.tag_underline_size(2)
srwm.cfg.tag_underline_offset_from_bar_bottom(0)
srwm.cfg.tag_underline_for_all_tags(false)

-- Misc
srwm.cfg.px_till_snapping_to_screen_edge(32)

--------------------------------------------------------------------------------
-- Startups
--------------------------------------------------------------------------------

-- srwm.spawn_once("picom -b")
-- srwm.spawn_once("feh --bg-scale ~/wallpaper.jpg")
-- srwm.spawn_once("nm-applet &")

--------------------------------------------------------------------------------
-- Keybindings
--------------------------------------------------------------------------------

-- Mod4 = SUPER
-- Mod1 = Alt

--------------------------------------------------------------------------------
-- Window Management
--------------------------------------------------------------------------------

srwm.key.bind("Mod4", "q", function()
	srwm.window.kill()
end)
srwm.key.bind("Mod4", "w", function()
	srwm.window.toggle_floating()
end)
srwm.key.bind("Mod4", "f", function()
	srwm.window.toggle_fullscreen()
end)
srwm.key.bind("Mod4", "Down", function()
	srwm.window.focus(1)
end)
srwm.key.bind("Mod4", "Up", function()
	srwm.window.focus(-1)
end)

--------------------------------------------------------------------------------
-- Tag Management
--------------------------------------------------------------------------------

srwm.key.bind("Mod4", "Left", function()
	srwm.tag.shift_view(-1)
end)
srwm.key.bind("Mod4", "Right", function()
	srwm.tag.shift_view(1)
end)
srwm.key.bind("Mod4+Ctrl", "Left", function()
	srwm.tag.view_prev()
end)
srwm.key.bind("Mod4+Ctrl", "Right", function()
	srwm.tag.view_next()
end)
srwm.key.bind("Mod4+Shift", "comma", function()
	srwm.tag.move_to_monitor(-1)
end)
srwm.key.bind("Mod4+Shift", "period", function()
	srwm.tag.move_to_monitor(1)
end)

-- View previous tag
srwm.key.bind("Mod4", "Tab", function()
	srwm.tag.view(0)
end)

-- Workspaces 1-9
for i = 1, 9 do
	local key = tostring(i)
	srwm.key.bind("Mod4", key, function()
		srwm.tag.view(i)
	end) -- SUPER + <numbers> to change tags
	srwm.key.bind("Mod4+Shift", key, function()
		srwm.tag.move_window_to(i)
	end) -- SUPER + SHIFT + <numbers> to move windows to specified tags
end

--------------------------------------------------------------------------------
--- Misc
--------------------------------------------------------------------------------

-- Volume Control
srwm.key.bind("", "XF86AudioRaiseVolume", function()
	srwm.spawn("wpctl set-volume @DEFAULT_AUDIO_SINK@ 2%+")
end)

srwm.key.bind("", "XF86AudioLowerVolume", function()
	srwm.spawn("wpctl set-volume @DEFAULT_AUDIO_SINK@ 2%-")
end)

srwm.key.bind("", "XF86AudioMute", function()
	srwm.spawn("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle")
end)

-- Brightness Control
srwm.key.bind("", "XF86MonBrightnessUp", function()
	srwm.spawn("brightnessctl set 5%+")
end)

srwm.key.bind("", "XF86MonBrightnessDown", function()
	srwm.spawn("brightnessctl set 5%-")
end)

-- General WM Control
srwm.key.bind("Mod4+Shift", "r", function()
	srwm.restart()
end)

srwm.key.bind("Mod4", "BackSpace", function()
	srwm.quit()
end)

srwm.key.bind("Mod4", "Return", function()
	srwm.spawn("alacritty")
end)

--------------------------------------------------------------------------------
-- Status Bar Setup
--------------------------------------------------------------------------------

-- Set fonts
srwm.bar.fonts("JetBrainsMonoNerdFont:size=13")

-- Theme: nested tables set WM core colors, simple strings set widget palette
srwm.bar.theme({
	-- WM Core colors (fg, bg, border)
	normal = { fg = "#cdcdcd", bg = "#252530", border = "#606079" }, -- should be moved to srwm.theme
	selected = { fg = "#cdcdcd", bg = "#6e94b2", border = "#6e94b2" }, -- should be moved to srwm.theme
	tab_selected = { fg = "#252530", bg = "#aeaed1", border = "#aeaed1" },
	button_prev = { fg = "#7fa563", bg = "#252530", border = "#252530" },
	button_next = { fg = "#f3be7c", bg = "#252530", border = "#252530" },
	button_close = { fg = "#d8647e", bg = "#252530", border = "#252530" },
	tab_normal = { fg = "#cdcdcd", bg = "#252530", border = "#252530" },
	title = { fg = "#d7d7d7", bg = "#252530", border = "#252530" },
	inactive_tag = { fg = "#606079", bg = "#252530", border = "#252530" },
	tag_1 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	tag_2 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	tag_3 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },
	tag_4 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	tag_5 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	tag_6 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },
	tag_7 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	tag_8 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	tag_9 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },

	-- Widget palette (used as {name} in shell scripts)
	purple = "#bebeda",
	darkpurple = "#aeaed1",
	black = "#252530",
	green = "#99b782",
	darkgreen = "#7fa563",
	white = "#d7d7d7",
	grey = "#606079",
	blue = "#8ba9c1",
	darkblue = "#6e94b2",
	red = "#e08398",
	darkred = "#d8647e",
	yellow = "#f5cb96",
	darkyellow = "#f3be7c",
	pink = "#bb9dbd",
})

-- Tag Preview behavior
srwm.bar.tags.highlight_occupied_only(true)

-- Register widget shell scripts (paths are relative to ~/.config/srwm/)
srwm.bar.widget("brightness", "widgets/brightness.sh")
srwm.bar.widget("volume", "widgets/volume.sh")
srwm.bar.widget("wifi", "widgets/wifi.sh")
srwm.bar.widget("clock", "widgets/clock.sh")
srwm.bar.widget("battery", "widgets/battery.sh")
srwm.bar.widget("gap", "widgets/gap.sh")

-- Define the layout left-to-right
srwm.bar.layout("brightness", "gap", "volume", "gap", "wifi", "gap", "clock", "gap", "battery")

-- Set polling interval (seconds)
srwm.bar.interval(1)

-- Start the status bar loop (blocks this execution thread)
srwm.bar.run()
