#!/bin/sh
# srwm widget: brightness

. "${SRWM_THEME:-}"

val=$(cat /sys/class/backlight/*/brightness 2>/dev/null)
[ -z "$val" ] && exit 0

printf "^c%s^^b%s^   ^b%s^ %.0f%%\n" "$black" "$red" "$darkred" "$val"
