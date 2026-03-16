/* See LICENSE file for copyright and license details. */

#include <X11/XF86keysym.h>
// theme
#include "themes/vague.h"

/* appearance - configurable via Lua */
extern int layout_mode;
extern unsigned int borderpx;
extern unsigned int px_till_snapping_to_screen_edge;
extern unsigned int gaps;
extern unsigned int systraypinning;
extern unsigned int systrayspacing;
extern int systray_enable;
extern int showbar;
extern int bar_horizontal_padding;
extern int bar_vertical_padding;
extern int tab_height;
extern int tab_tile_vertical_padding;
extern int tab_tile_inner_padding_horizontal;
extern int tab_tile_outer_padding_horizontal;
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
  {MODKEY | ControlMask, KEY, tag, {.ui = 1 << TAG}},
