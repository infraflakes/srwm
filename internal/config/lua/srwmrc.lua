-- Main configuration for srwm
local bar = require("bar")

--------------------------------------------------------------------------------
-- Keybindings
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

srwm.key.bind("Mod4", "space", function()
	srwm.spawn("rofi -show drun")
end)

srwm.key.bind("Mod4", "Return", function()
	srwm.spawn("alacritty")
end)

--------------------------------------------------------------------------------
-- Status Bar Setup
--------------------------------------------------------------------------------

-- Start the status bar loop (blocks this execution thread)
bar.run()
