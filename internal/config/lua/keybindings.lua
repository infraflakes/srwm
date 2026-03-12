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
