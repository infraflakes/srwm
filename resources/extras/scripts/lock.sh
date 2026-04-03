#!/bin/sh
# Blur-lock: screenshot, blur, lock
grim -l 0 /tmp/lockscreen.png
ffmpeg -y -i /tmp/lockscreen.png -vf "boxblur=8:2" /tmp/lockblur.png 2>/dev/null
swaylock -f -i /tmp/lockblur.png
