# vxwm - Versatile X Window Manager for x11 forked from dwm

# Features
vxwm is dwm on steroids, it can be as lightweight as you want.
It has some patches for dwm pre installed that can be enabled/disabled by switching 0 to 1 or vice versa. (like in dwm-flexipatch)
It also has some unique features like 8 sided resize, enhanced toggle floating, warp cursor and more! (They also can be toggled)
Check modules.def.h for full list of features.

But the main feature of vxwm, is infinite tags.
 
Most tiling window managers (like the default dwm) treat your screen like a slide-projector. You click a button, and the current "slide" is swapped for another. If an window is off-screen, it doesn't exist.

With infinite tags enabled, vxwm treats your screen like a magnifying glass over a giant wooden desk.

The Canvas is Infinite
Your windows aren't "on" tags. They are placed on a massive, invisible surface. Your monitor is just a small window through which you look at that surface.

Move the View, Not Just the Windows
Instead of managing "layers" or "hidden states," you manage position.

Want more space? Slide the view over.
Can't find a window? Swicth your focus to it, and the world slides until that window is right under your nose.
Lost? Hit the "homecanvas" keybind to snap your view back to the start.
Even though this sounds complex, it is actually pretty lightweight ~250 l.o.c, and is very easy to use.

## Requirements

In order to build vxwm you need the Xlib header files.

# Getting Started:

## Installation

Clone this repository and cd into it.

    git clone https://codeberg.org/wh1tepearl/vxwm.git
    cd vxwm

Edit config.mk to match your local setup (vxwm is installed into
the /usr/local namespace by default).

Enable Xinerama if you need it by uncommenting Xinerama libs in config.mk.

Afterwards enter the following command to build and install vxwm (if
necessary as root):

    make clean install


## Running vxwm

Add the following line to your .xinitrc to start vxwm using startx:

    exec vxwm

If you want to restart vxwm without losing your session 
or for hot configuration reload, add something like this to your .xinitrc:

    vxwm &
    exec sleep infinity

And then for restarting vxwm just kill vxwm's process and start it again or use rvx utility.

In order to connect vxwm to a specific display, make sure that
the DISPLAY environment variable is set correctly, e.g.:

    DISPLAY=foo.bar:1 exec vxwm

(This will start vxwm on display :1 of the host foo.bar.)

In order to display status info in the bar, you can do something
like this in your .xinitrc:

    while xsetroot -name "`date` `uptime | sed 's/.*,//'`"
    do
    	sleep 1
    done &
    exec vxwm


## Configuration

The configuration of vxwm is done by editing config.h and modules.h to
match your preferences and (re)compiling the source code.

## Acknowledgements

vxwm was made in solo by a linux enthusiast wh1tepearl, many thanks to suckless.org and the [dwm] developers for making dwm in first place.
Thanks 5element developer and hevel wayland compositor developers for inspiration of infinite tags.

Also try:

hevel wayland compositor: https://git.sr.ht/~dlm/hevel

5element: https://hg.sr.ht/~umix11/5element
