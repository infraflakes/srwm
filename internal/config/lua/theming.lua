--------------------------------------------------------------------------------
-- Theming
--------------------------------------------------------------------------------

-- Theme: nested tables set WM core colors, simple strings set widget palette
srwm.bar.theme({
	normal = { fg = "#cdcdcd", bg = "#252530", border = "#606079" }, -- should be moved to srwm.theme
	selected = { fg = "#cdcdcd", bg = "#6e94b2", border = "#6e94b2" }, -- should be moved to srwm.theme
	tab_selected = { fg = "#252530", bg = "#aeaed1", border = "#aeaed1" },
	button_prev = { fg = "#7fa563", bg = "#252530", border = "#252530" },
	button_next = { fg = "#f3be7c", bg = "#252530", border = "#252530" },
	button_close = { fg = "#d8647e", bg = "#252530", border = "#252530" },
	tab_normal = { fg = "#cdcdcd", bg = "#252530", border = "#252530" },
	title = { fg = "#d7d7d7", bg = "#252530", border = "#252530" },
	highlight_occupied_only = { fg = "#606079", bg = "#252530", border = "#252530" },
	ws_1 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	ws_2 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	ws_3 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },
	ws_4 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	ws_5 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	ws_6 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },
	ws_7 = { fg = "#6e94b2", bg = "#252530", border = "#252530" },
	ws_8 = { fg = "#aeaed1", bg = "#252530", border = "#252530" },
	ws_9 = { fg = "#bb9dbd", bg = "#252530", border = "#252530" },

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

-- Workspace Preview behavior
srwm.bar.workspaces.highlight_occupied_only(true)

-- Register widget shell scripts (paths are relative to ~/.config/srwm/)
srwm.bar.widget("brightness", "widgets/brightness.sh")
srwm.bar.widget("volume", "widgets/volume.sh")
srwm.bar.widget("wifi", "widgets/wifi.sh")
srwm.bar.widget("clock", "widgets/clock.sh")
srwm.bar.widget("battery", "widgets/battery.sh")
srwm.bar.widget("gap", "widgets/gap.sh")

-- Define the layout left-to-right
srwm.bar.layout("brightness", "gap", "volume", "gap", "wifi", "gap", "clock", "gap", "battery")
