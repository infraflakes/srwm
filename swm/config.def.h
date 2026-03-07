/* See LICENSE file for copyright and license details. */

#include <X11/XF86keysym.h>
// theme
#include "themes/vague.h"

/* appearance */
static const unsigned int borderpx = 0; /* border pixel of windows */
static const unsigned int default_border =
    0; /* to switch back to default border after dynamic border resizing via
          keybinds */
static const unsigned int snap = 32;     /* snap pixel */
static const unsigned int gap_value = 0; /* horiz inner gap between windows */
static const unsigned int gappih =
    gap_value; /* horiz inner gap between windows */
static const unsigned int gappiv =
    gap_value; /* vert inner gap between windows */
static const unsigned int gappoh =
    gap_value; /* horiz outer gap between windows and screen edge */
static const unsigned int gappov =
    gap_value; /* vert outer gap between windows and screen edge */
static const int smartgaps =
    0; /* 1 means no outer gap when there is only one window */
static const unsigned int systraypinning =
    0; /* 0: sloppy systray follows selected monitor, >0: pin systray to monitor
          X */
static const unsigned int systrayspacing = 2; /* systray spacing */
static const int systraypinningfailfirst =
    1; /* 1: if pinning fails,display systray on the 1st monitor,False: display
          systray on last monitor*/
static const int showsystray = 1; /* 0 means no systray */
static const int showbar = 1;     /* 0 means no bar */
static const int showtab = showtab_auto;
static const int toptab = 1;   /* 0 means bottom tab */
static const int floatbar = 1; /* 1 means the bar will float(don't have
                                  padding),0 means the bar have padding */
static const int topbar = 1;   /* 0 means bottom bar */
static const int horizpadbar = 10;
static const int vertpadbar = 15;
static const int vertpadtab = 35;
static const int horizpadtabi = 15;
static const int horizpadtabo = 15;
static const int scalepreview = 4;
static const int tag_preview = 0; /* 1 means enable, 0 is off */
static const int colorfultag =
    1; /* 0 means use SchemeSel for selected non vacant tag */

static const char* volup[] = {"swmctl", "volume", "up", "2", NULL};
static const char* voldown[] = {"swmctl", "volume", "down", "2", NULL};
static const char* volmute[] = {"swmctl", "volume", "mute", NULL};
static const char* brightup[] = {"swmctl", "brightness", "up", "5", NULL};
static const char* brightdown[] = {"swmctl", "brightness", "down", "5", NULL};

static const char* quit_swm[] = {"swmctl", "quit", NULL};
static const int new_window_attach_on_end =
    1; /*  1 means the new window will attach on the end; 0 means the new window
          will attach on the front,default is front */
#define ICONSIZE 20   /* icon size */
#define ICONSPACING 8 /* space between icon and title */

static const char* fonts[] = {"JetBrainsMonoNerdFont:size=13"};

static const char* colors[][3] = {
    /*            fg       bg      border */
    [SchemeNorm] = {gray3, black, gray2},
    [SchemeSel] = {gray3, blue, blue},
    [SchemeTitle] = {white, black, black},  // active window title
    [TabSel] = {black, purple, black},
    [TabNorm] = {gray3, black, black},
    [SchemeTag] = {gray2, black, black},
    [SchemeTag1] = {blue, black, black},
    [SchemeTag2] = {purple, black, black},
    [SchemeTag3] = {pink, black, black},
    [SchemeBtnPrev] = {green, black, black},
    [SchemeBtnNext] = {yellow, black, black},
    [SchemeBtnClose] = {red, black, black},
};

/* tagging */
static char* tags[] = {"1", "2", "3", "4", "5"};

static const int tagschemes[] = {SchemeTag1, SchemeTag2, SchemeTag3,
                                 SchemeTag2, SchemeTag1, SchemeTag2,
                                 SchemeTag3, SchemeTag1, SchemeTag2};

static const unsigned int ulinepad =
    5; /* horizontal padding between the underline and tag */
static const unsigned int ulinestroke =
    2; /* thickness / height of the underline */
static const unsigned int ulinevoffset =
    0; /* how far above the bottom of the bar the line should appear */
static const int ulineall =
    0; /* 1 to show underline on all tags, 0 for just the active ones */

static const Rule rules[] = {
    /* xprop(1):
     *	WM_CLASS(STRING) = instance, class
     *	WM_NAME(STRING) = title
     */
    /* class      instance    title       tags mask     iscentered   isfloating
       monitor */
    { "Toolkit",  NULL,       "Picture-in-Picture",   0,         1,          -1 },
    { "firefox",  NULL,       "Picture-in-Picture",   0,         1,          -1 },
    { "Chromium", NULL,       "Picture-in-Picture",   0,         1,          -1 },
};

/* layout(s) */
static const int resizehints =
    0; /* 1 means respect size hints in tiled resizals */
static const int lockfullscreen =
    1; /* 1 will force focus on the fullscreen window */

#include "functions.h"

/* function declarations */
static void tagtonext(const Arg* arg);
static void tagtoprev(const Arg* arg);
static void viewnext(const Arg* arg);
static void viewprev(const Arg* arg);
static unsigned int nexttag(void);
static unsigned int prevtag(void);

/* key definitions */
#define MODKEY Mod4Mask
#define ALTKEY Mod1Mask
#define TAGKEYS(KEY, TAG)                                        \
  {MODKEY, KEY, view, {.ui = 1 << TAG}},                         \
      {MODKEY | ControlMask, KEY, toggleview, {.ui = 1 << TAG}}, \
      {MODKEY | ShiftMask, KEY, tag, {.ui = 1 << TAG}},          \
      {MODKEY | ControlMask | ShiftMask, KEY, toggletag, {.ui = 1 << TAG}},

/* helper for spawning shell commands in the pre dwm-5.0 fashion */
#define SHCMD(cmd)                                 \
  {                                                \
    .v = (const char*[]) { "sh", "-c", cmd, NULL } \
  }

/* commands */

static const Key keys[] = {
    /* modifier                         key         function        argument */

    // brightness and audio
    {0, XF86XK_AudioRaiseVolume, spawn, {.v = volup}},
    {0, XF86XK_AudioLowerVolume, spawn, {.v = voldown}},
    {0, XF86XK_AudioMute, spawn, {.v = volmute}},
    {0, XF86XK_MonBrightnessUp, spawn, {.v = brightup}},
    {0, XF86XK_MonBrightnessDown, spawn, {.v = brightdown}},

    // screenshot fullscreen and cropped
    {MODKEY | ALTKEY, XK_s, spawn, SHCMD("flameshot full")},
    {MODKEY | ShiftMask, XK_s, spawn, SHCMD("flameshot gui")},
    {MODKEY, XK_space, spawn, SHCMD("swmctl launcher")},
    {MODKEY, XK_Return, spawn, SHCMD("alacritty")},
    {MODKEY, XK_l, spawn, SHCMD("slock")},
    {MODKEY, XK_v, spawn, SHCMD("copyq menu")},
    {MODKEY, XK_BackSpace, spawn, {.v = quit_swm}},  // quit swm MOD+backspace
    // restart
    {MODKEY | ShiftMask, XK_r, restart, {0}},

    // toggle stuff
    {MODKEY, XK_w, togglefloating, {0}},
    {MODKEY, XK_f, togglefullscr, {0}},

    {MODKEY, XK_Down, focusstack, {.i = +1}},
    {MODKEY, XK_Up, focusstack, {.i = -1}},

    // shift view
    {MODKEY, XK_Left, shiftview, {.i = -1}},
    {MODKEY, XK_Right, shiftview, {.i = +1}},

    {MODKEY | ControlMask, XK_Left, tagtoprev, {0}},
    {MODKEY | ControlMask, XK_Right, tagtonext, {0}},
    {MODKEY, XK_Tab, view, {0}},

    // { MODKEY|ControlMask,               XK_i,       incrgaps,       {.i = +10 } },
    // { MODKEY|ControlMask,               XK_d,       incrgaps,       {.i = -10 } },

    // kill window
    {MODKEY, XK_q, killclient, {0}},

    TAGKEYS(XK_1, 0) TAGKEYS(XK_2, 1) TAGKEYS(XK_3, 2) TAGKEYS(XK_4, 3)
        TAGKEYS(XK_5, 4) TAGKEYS(XK_6, 5) TAGKEYS(XK_7, 6) TAGKEYS(XK_8, 7)
            TAGKEYS(XK_9, 8)};

/* button definitions */
static const Button buttons[] = {
    {ClkStatusText, 0, Button2, spawn, SHCMD("ghostty")},
    {ClkClientWin, MODKEY, Button1, moveorplace, {.i = 0}},
    {ClkClientWin, MODKEY, Button2, togglefloating, {0}},
    {ClkClientWin, MODKEY, Button3, resizemouse, {0}},
    {ClkTagBar, 0, Button1, view, {0}},
    {ClkTagBar, 0, Button3, toggleview, {0}},
    {ClkTagBar, MODKEY, Button1, tag, {0}},
    {ClkTagBar, MODKEY, Button3, toggletag, {0}},
    {ClkTabBar, 0, Button1, focuswin, {0}},
    {ClkTabBar, 0, Button1, focuswin, {0}},
    {ClkTabPrev, 0, Button1, movestack, {.i = -1}},
    {ClkTabNext, 0, Button1, movestack, {.i = +1}},
    {ClkTabClose, 0, Button1, killclient, {0}},
};
