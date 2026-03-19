#!/bin/sh
# srwm widget: brightness

val=$(cat /sys/class/backlight/*/brightness 2>/dev/null)
max=$(cat /sys/class/backlight/*/max_brightness 2>/dev/null)
[ -z "$val" ] && exit 0

printf "^c{black}^^b{red}^   ^b{darkred}^ %.0f%%\n" "$(( val * 100 / max ))"
