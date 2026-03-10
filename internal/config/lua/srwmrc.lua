-- Example configuration for srwm!

-- You can split your config to other <name>.lua files and include them as
-- include("<name>")
-- without .lua extension

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

-- Define theme variables for widget template replacement (e.g. {purple} -> #hex)
srwm.bar.theme({
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
