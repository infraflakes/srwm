#!/bin/sh
# srwm widget: clock

. "${SRWM_THEME:-}"

printf "^c%s^ ^b%s^ 󱑆  ^b%s^ %s" "$black" "$purple" "$darkpurple" "$(date '+%H:%M:%S')"
