/* See LICENSE file for copyright and license details. */

#include <X11/XF86keysym.h>
// theme
#include "themes/vague.h"

/* appearance - configurable via Lua */
extern unsigned int borderpx;
extern unsigned int px_till_snapping_to_screen_edge;
extern unsigned int gaps;
extern unsigned int systraypinning;
extern unsigned int systrayspacing;
extern int systray_enable;
extern int showbar;
extern int bar_horizontal_padding;
extern int bar_vertical_padding;
extern int tab_vertical_padding;
extern int tab_in_horizontal_padding;
extern int tab_out_horizontal_padding;
extern int tag_preview_size;
extern int tag_preview_enable;
extern unsigned int tag_underline_padding;
extern unsigned int tag_underline_size;
extern unsigned int tag_underline_offset_from_bar_bottom;
extern int tag_underline_for_all_tags;

// Needs refactor
extern int toptab;
extern int topbar;

// Theming
#define ICONSIZE 20
#define ICONSPACING 8
extern const char* fonts[];
extern const char* colors[][3];

// tagging
extern char* tags[];
extern int tagschemes[];

extern const Rule rules[];

#include "functions.h"
/* function declarations */
static void tagtonext(const Arg* arg);
static void tagtoprev(const Arg* arg);
static unsigned int nexttag(void);
static unsigned int prevtag(void);

/* key definitions */
#define MODKEY Mod4Mask
#define ALTKEY Mod1Mask
#define TAGKEYS(KEY, TAG)                              \
  {MODKEY, KEY, view, {.ui = 1 << TAG}},               \
  {MODKEY | ControlMask, KEY, tag, {.ui = 1 << TAG}},  \

static const Key keys[] = {

    {MODKEY, XK_q, killclient, {0}},

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
