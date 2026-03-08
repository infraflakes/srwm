-- Default status bar logic for srwm

local theme = {
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
}

-- helper to read the first line of a file/command
local function read_first_line(cmd)
	local f = io.popen(cmd)
	if not f then
		return ""
	end
	local line = f:read("*l") or ""
	f:close()
	return line
end

-- helper to read all output
local function read_all(cmd)
	local f = io.popen(cmd)
	if not f then
		return ""
	end
	local txt = f:read("*a") or ""
	f:close()
	return txt
end

local function battery()
	local cap = read_first_line("cat /sys/class/power_supply/BAT0/capacity 2>/dev/null")
	if cap == "" then
		return ""
	end
	return string.format("^c%s^ ^b%s^ 󱐋 ^b%s^ %s ", theme.black, theme.yellow, theme.darkyellow, cap)
end

local function brightness()
	local b = read_first_line("cat /sys/class/backlight/*/brightness 2>/dev/null")
	if b == "" then
		return ""
	end
	local pct = tonumber(b)
	if not pct then
		pct = 0
	end
	return string.format("^c%s^^b%s^   ^b%s^ %d%%", theme.black, theme.red, theme.darkred, pct)
end

local function volume()
	local wp_info = read_first_line("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null")
	if wp_info == "" then
		return string.format("^c%s^ ^b%s^ 󰝟 Error", theme.black, theme.red)
	end

	local vol_raw = string.match(wp_info, "Volume:%s*(%d%.%d+)")
	if not vol_raw then
		vol_raw = "0"
	end
	local vol_int = math.floor(tonumber(vol_raw) * 100)

	local is_muted = string.match(wp_info, "%[MUTED%]")
	if is_muted then
		return string.format("^c%s^ ^b%s^ 󰝟 Muted", theme.black, theme.red)
	end

	local icon = "󰕾"
	if vol_int == 0 then
		icon = "󰝟"
	elseif vol_int <= 33 then
		icon = "󰕿"
	elseif vol_int <= 66 then
		icon = "󰖀"
	end

	return string.format("^c%s^ ^b%s^ %s ^b%s^ %d%%", theme.black, theme.green, icon, theme.darkgreen, vol_int)
end

local function wlan()
	-- Get interface name
	local iface = read_first_line("find /sys/class/net -name 'wl*' -type l | head -1 | xargs basename 2>/dev/null")
	if iface == "" then
		return ""
	end

	local state = read_first_line("cat /sys/class/net/" .. iface .. "/operstate 2>/dev/null")
	if state == "down" or state == "" then
		return string.format("^c%s^ ^b%s^ 󰤭  ^b%s^ Disconnected", theme.black, theme.blue, theme.darkblue)
	end

	local iw_info = read_all("iw dev " .. iface .. " info 2>/dev/null")
	local ssid = string.match(iw_info, "ssid ([^\n]+)")
	if not ssid then
		ssid = "Unknown"
	end

	return string.format("^c%s^ ^b%s^ 󰤨  ^b%s^ %s", theme.black, theme.blue, theme.darkblue, ssid)
end

local function clock()
	local t = os.date("%H:%M:%S")
	return string.format("^c%s^ ^b%s^ 󱑆  ^b%s^ %s", theme.black, theme.purple, theme.darkpurple, t)
end

local function gap()
	return string.format("^c%s^ ^b%s^", theme.black, theme.black)
end

local function refresh()
	local parts = {
		brightness(),
		gap(),
		volume(),
		gap(),
		wlan(),
		gap(),
		clock(),
		gap(),
		battery(),
	}

	-- filter empty strings
	local text = ""
	for _, p in ipairs(parts) do
		text = text .. p
	end

	srwm.set_status(text)
end

while true do
	refresh()
	srwm.sleep(1)
end
