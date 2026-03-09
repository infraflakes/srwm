/* See LICENSE file for copyright and license details. */

#include <X11/XF86keysym.h>
// theme
#include "themes/vague.h"

/* appearance */
static const unsigned int borderpx = 0; /* border pixel of windows */
static const unsigned int default_border = 0; /* to switch back to default border after dynamic border resizing via keybinds */
static const unsigned int attach_to_screen_edge_px = 32;     /* snap pixel */
static const unsigned int gaps = 0; /* set up gaps */
static const unsigned int systraypinning = 0;   /* 0: sloppy systray follows selected monitor, >0: pin systray to monitor X */
static const unsigned int systrayspacing = 2; /* systray spacing */
static const int showsystray = 1; /* 0 means no systray */
static const int showbar = 1;     /* 0 means no bar */
static const int toptab = 1;   /* 0 means bottom tab */
static const int topbar = 1;   /* 0 means bottom bar */
static const int horizpadbar = 10;
static const int vertpadbar = 0;
static const int vertpadtab = 35;
static const int horizpadtabi = 15;
static const int horizpadtabo = 15;
static const int scalepreview = 4;
static const int tag_preview = 0; /* 1 means enable, 0 is off */
static const int colorfultag = 1; /* 0 means use SchemeSel for selected non vacant tag */
static const unsigned int ulinepad = 5; /* horizontal padding between the underline and tag */
static const unsigned int ulinestroke = 2; /* thickness / height of the underline */
static const unsigned int ulinevoffset = 0; /* how far above the bottom of the bar the line should appear */
static const int ulineall = 0; /* 1 to show underline on all tags, 0 for just the active ones */
static const int new_window_attach_on_end = 1; /*  1 means the new window will attach on the end; 0 means the new window will attach on the front,default is front */
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

#include "functions.h"
/* function declarations */
static void tagtonext(const Arg* arg);
static void tagtoprev(const Arg* arg);
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
#define SHCMD(cmd) {.v = (const char*[]) { "sh", "-c", cmd, NULL }}
static const Key keys[] = {

    {MODKEY, XK_q, killclient, {0}},
    {MODKEY | ALTKEY, XK_s, spawn, SHCMD("flameshot full")},
    {MODKEY | ShiftMask, XK_s, spawn, SHCMD("flameshot gui")},
    {MODKEY, XK_space, spawn, SHCMD("srwmctl launcher")},
    {MODKEY, XK_l, spawn, SHCMD("slock")},
    {MODKEY, XK_v, spawn, SHCMD("copyq menu")},

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
    {MODKEY|ShiftMask, XK_comma,  move_tag_to_monitor, {.i = -1 } },
    {MODKEY|ShiftMask, XK_period, move_tag_to_monitor, {.i = +1 } },

    TAGKEYS(XK_1, 0)
    TAGKEYS(XK_2, 1) 
    TAGKEYS(XK_3, 2) 
    TAGKEYS(XK_4, 3)
    TAGKEYS(XK_5, 4) 
    TAGKEYS(XK_6, 5) 
    TAGKEYS(XK_7, 6) 
    TAGKEYS(XK_8, 7)
    TAGKEYS(XK_9, 8)
};

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
