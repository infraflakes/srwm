-------------------------------------------------------------------------------
-- Theming
-------------------------------------------------------------------------------

-- Window border colors
srwm.window.border.active("#6e94b2")
srwm.window.border.inactive("#606079")

-- Bar background
srwm.bar.bg("#252530")

-- Theme
srwm.bar.theme({
	title = { "#d7d7d7", "#252530" },
	tab_selected = { "#252530", "#aeaed1" },
	tab_normal = { "#cdcdcd", "#252530" },
	inactive_ws = { "#606079", "#252530" },
	ws_1 = { "#6e94b2", "#252530" },
	ws_2 = { "#aeaed1", "#252530" },
	ws_3 = { "#bb9dbd", "#252530" },
	ws_4 = { "#aeaed1", "#252530" },
	ws_5 = { "#6e94b2", "#252530" },
	ws_6 = { "#bb9dbd", "#252530" },
	ws_7 = { "#6e94b2", "#252530" },
	ws_8 = { "#aeaed1", "#252530" },
	ws_9 = { "#bb9dbd", "#252530" },

	-- Tab bar buttons
	button_prev = "#7fa563",
	button_next = "#f3be7c",
	button_close = "#d8647e",

	-- Widget palette
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

