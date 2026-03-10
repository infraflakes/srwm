-- Example configuration for srwm!

-- You can split your config to other <name>.lua files and include them as
-- include("<name>")
-- without .lua extension

--------------------------------------------------------------------------------
-- Configuration
--------------------------------------------------------------------------------
-- Note: These take effect on restart (srwm.restart())

-- Appearance
srwm.cfg.borderpx(0)
srwm.cfg.gaps(0)
srwm.cfg.showbar(true)
srwm.cfg.topbar(true)
srwm.cfg.toptab(true)
srwm.cfg.colorfultag(true)

-- Bar
srwm.cfg.bar_horizontal_padding(10)
srwm.cfg.bar_vertical_padding(15)

-- Tab
srwm.cfg.tab_vertical_padding(35)
srwm.cfg.tab_in_horizontal_padding(15)
srwm.cfg.tab_out_horizontal_padding(15)

-- Systray
srwm.cfg.systray_enable(true)
srwm.cfg.systray_spacing(2)
srwm.cfg.systray_pinning(0)

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
srwm.cfg.new_window_appear_on_end(true)

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
	normal = { fg = "#cdcdcd", bg = "#252530", border = "#606079" },
	selected = { fg = "#cdcdcd", bg = "#6e94b2", border = "#6e94b2" },
	title = { fg = "#d7d7d7", bg = "#252530", border = "#252530" },
	tab_selected = { fg = "#252530", bg = "#aeaed1", border = "#aeaed1" },
	tab_normal = { fg = "#cdcdcd", bg = "#252530", border = "#252530" },
	tag = { fg = "#606079", bg = "#252530", border = "#252530" },
	tag1 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	tag2 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	tag3 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },
	button_prev = { fg = "#7fa563", bg = "#252530", border = "#252530" },
	button_next = { fg = "#f3be7c", bg = "#252530", border = "#252530" },
	button_close = { fg = "#d8647e", bg = "#252530", border = "#252530" },

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
})

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
