/* bridge.c — Thin lifecycle wrapper around dwm.c for Go */
#include "bridge.h"
#include "wm.h"
#include <X11/Xlib.h>
#include <locale.h>
#include <stdio.h>
#include <string.h>

/* Internal flag: distinguishes restart (running=0) from quit */
static int restart_requested = 0;

KeySym srwm_string_to_keysym(const char *name) {
  return XStringToKeysym(name);
}

int srwm_init_display(void) {
  /* Reset for re-init on restart */
  running = 1;
  restart_requested = 0;

  /* Initialize threads for multi-threaded Xlib access (Lua handles keys) */
  XInitThreads();

  if (!setlocale(LC_CTYPE, "") || !XSupportsLocale())
    fputs("warning: no locale support\n", stderr);

  if (!(dpy = XOpenDisplay(NULL))) {
    fputs("srwm: cannot open display\n", stderr);
    return -1;
  }

  checkotherwm();
  return 0;
}

void srwm_init_setup(void) {
  setup();
  scan();
}

void srwm_run(void) {
  run(); /* blocks until running == 0 */
}

void srwm_cleanup(void) {
  cleanup();
  XCloseDisplay(dpy);
  dpy = NULL;
}

void srwm_quit(void) {
  restart_requested = 0;
  running = 0;
}

int srwm_should_restart(void) {
  return restart_requested;
}

void srwm_set_status(const char *text) {
  if (!dpy) return;
  XStoreName(dpy, DefaultRootWindow(dpy), text);
  XFlush(dpy);
}

void srwm_request_restart(void) {
  restart_requested = 1;
  running = 0;
}

/* Dynamic keys defined in dwm.c */
extern void add_dynamic_key(unsigned int mod, KeySym keysym, int id);
extern void clear_dynamic_keys(void);
extern void grabkeys(void);

void srwm_add_keybinding(unsigned int mod, KeySym keysym, int id) {
  add_dynamic_key(mod, keysym, id);
}

void srwm_clear_keybindings(void) {
  clear_dynamic_keys();
}

void srwm_grabkeys(void) {
  grabkeys();
}

unsigned int srwm_get_borderpx(void) { return borderpx; }
void srwm_set_borderpx(unsigned int v) { borderpx = v; }

unsigned int srwm_get_px_till_snapping_to_screen_edge(void) { return px_till_snapping_to_screen_edge; }
void srwm_set_px_till_snapping_to_screen_edge(unsigned int v) { px_till_snapping_to_screen_edge = v; }

unsigned int srwm_get_gaps(void) { return gaps; }
void srwm_set_gaps(unsigned int v) { gaps = v; }

unsigned int srwm_get_systraypinning(void) { return systraypinning; }
void srwm_set_systraypinning(unsigned int v) { systraypinning = v; }

unsigned int srwm_get_systrayspacing(void) { return systrayspacing; }
void srwm_set_systrayspacing(unsigned int v) { systrayspacing = v; }

int srwm_get_systray_enable(void) { return systray_enable; }
void srwm_set_systray_enable(int v) { systray_enable = v; }

int srwm_get_showbar(void) { return showbar; }
void srwm_set_showbar(int v) { showbar = v; }

int srwm_get_bar_horizontal_padding(void) { return bar_horizontal_padding; }
void srwm_set_bar_horizontal_padding(int v) { bar_horizontal_padding = v; }

int srwm_get_bar_vertical_padding(void) { return bar_vertical_padding; }
void srwm_set_bar_vertical_padding(int v) { bar_vertical_padding = v; }

int srwm_get_tab_height(void) { return tab_height; }
void srwm_set_tab_height(int v) { tab_height = v; }

int srwm_get_tab_tile_vertical_padding(void) { return tab_tile_vertical_padding; }
void srwm_set_tab_tile_vertical_padding(int v) { tab_tile_vertical_padding = v; }

int srwm_get_tab_tile_inner_padding_horizontal(void) { return tab_tile_inner_padding_horizontal; }
void srwm_set_tab_tile_inner_padding_horizontal(int v) { tab_tile_inner_padding_horizontal = v; }

int srwm_get_tab_tile_outer_padding_horizontal(void) { return tab_tile_outer_padding_horizontal; }
void srwm_set_tab_tile_outer_padding_horizontal(int v) { tab_tile_outer_padding_horizontal = v; }

unsigned int srwm_get_tag_underline_padding(void) { return tag_underline_padding; }
void srwm_set_tag_underline_padding(unsigned int v) { tag_underline_padding = v; }

unsigned int srwm_get_tag_underline_size(void) { return tag_underline_size; }
void srwm_set_tag_underline_size(unsigned int v) { tag_underline_size = v; }

unsigned int srwm_get_tag_underline_offset_from_bar_bottom(void) { return tag_underline_offset_from_bar_bottom; }
void srwm_set_tag_underline_offset_from_bar_bottom(unsigned int v) { tag_underline_offset_from_bar_bottom = v; }

int srwm_get_tag_underline_for_all_tags(void) { return tag_underline_for_all_tags; }
void srwm_set_tag_underline_for_all_tags(int v) { tag_underline_for_all_tags = v; }

int srwm_get_toptab(void) { return toptab; }
void srwm_set_toptab(int v) { toptab = v; }

int srwm_get_topbar(void) { return topbar; }
void srwm_set_topbar(int v) { topbar = v; }


extern const char* fonts[];
void srwm_set_font(const char* font) { fonts[0] = font; }

extern const char* colors[][3];
void srwm_set_color(int scheme, int slot, const char* hex) {
  if (slot >= 0 && slot < 3) colors[scheme][slot] = strdup(hex);
}

extern char* tags[];
extern int tags_len;
void srwm_set_tag(int idx, const char* name) {
  if (idx >= 0 && idx < 9) tags[idx] = strdup(name);
}

void srwm_set_tags_len(int len) {
  if (len >= 0 && len <= 9) tags_len = len;
}

extern int tagschemes[];
void srwm_set_tagscheme(int idx, int scheme_idx) {
  if (idx >= 0 && idx < 9) tagschemes[idx] = scheme_idx;
}

/* srwm_action bindings mapping primitives to internal Arg structs */
void srwm_action_killclient(void) { killclient(&(Arg){0}); }
void srwm_action_togglefloating(void) { togglefloating(&(Arg){0}); }
void srwm_action_togglefullscr(void) { togglefullscr(&(Arg){0}); }
void srwm_action_focusstack(int dir) { focusstack(&(Arg){.i = dir}); centerwindowoncanvas(&(Arg){0}); }
void srwm_action_shiftview(int dir) { shiftview(&(Arg){.i = dir}); }
void srwm_action_tagtoprev(void) { tagtoprev(&(Arg){0}); }
void srwm_action_tagtonext(void) { tagtonext(&(Arg){0}); }
void srwm_action_move_tag_to_monitor(int dir) { move_tag_to_monitor(&(Arg){.i = dir}); }
void srwm_action_view(unsigned int mask) { view(&(Arg){.ui = mask}); }
void srwm_action_toggleview(unsigned int mask) { toggleview(&(Arg){.ui = mask}); }
void srwm_action_tag(unsigned int mask) { tag(&(Arg){.ui = mask}); }
void srwm_action_toggletag(unsigned int mask) { toggletag(&(Arg){.ui = mask}); }
void srwm_set_tag_colorful_occupied_only(int val) {tag_colorful_occupied_only = val;}
int srwm_get_layout_mode(void) { return layout_mode; }  
void srwm_set_layout_mode(int val) { layout_mode = val; }

void srwm_action_movecanvas(int dir) { movecanvas(&(Arg){.i = dir}); }  
void srwm_action_homecanvas(void) { homecanvas(&(Arg){0}); }  
void srwm_action_centerwindowoncanvas(void) { centerwindowoncanvas(&(Arg){0}); }  
void srwm_action_zoomcanvas(int dir) { zoomcanvas(&(Arg){.i = dir}); }
