--------------------------------------------------------------------------------
-- Theming
--------------------------------------------------------------------------------

-- Window border colors
srwm.window.border.active("#6e94b2")
srwm.window.border.inactive("#606079")

-- Tab bar theme
srwm.bar.theme({
	tab_selected = { "#252530", "#aeaed1" },
	tab_normal = { "#cdcdcd", "#252530" },
	button_prev = "#7fa563",
	button_next = "#f3be7c",
	button_close = "#d8647e",
})