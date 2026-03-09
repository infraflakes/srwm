#!/bin/sh
# srwm widget: battery

. "${SRWM_THEME:-}"

cap=$(cat /sys/class/power_supply/BAT0/capacity 2>/dev/null)
[ -z "$cap" ] && exit 0

printf "^c%s^ ^b%s^ 󱐋 ^b%s^ %s " "$black" "$yellow" "$darkyellow" "$cap"
