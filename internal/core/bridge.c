/* bridge.c — Thin lifecycle wrapper around dwm.c for Go */
#include "bridge.h"
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

/* Config getters and setters */
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
extern int toptab;
extern int topbar;
extern int new_window_appear_on_end;
extern int colorfultag;

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

int srwm_get_tab_vertical_padding(void) { return tab_vertical_padding; }
void srwm_set_tab_vertical_padding(int v) { tab_vertical_padding = v; }

int srwm_get_tab_in_horizontal_padding(void) { return tab_in_horizontal_padding; }
void srwm_set_tab_in_horizontal_padding(int v) { tab_in_horizontal_padding = v; }

int srwm_get_tab_out_horizontal_padding(void) { return tab_out_horizontal_padding; }
void srwm_set_tab_out_horizontal_padding(int v) { tab_out_horizontal_padding = v; }

int srwm_get_tag_preview_size(void) { return tag_preview_size; }
void srwm_set_tag_preview_size(int v) { tag_preview_size = v; }

int srwm_get_tag_preview_enable(void) { return tag_preview_enable; }
void srwm_set_tag_preview_enable(int v) { tag_preview_enable = v; }

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

int srwm_get_new_window_appear_on_end(void) { return new_window_appear_on_end; }
void srwm_set_new_window_appear_on_end(int v) { new_window_appear_on_end = v; }

int srwm_get_colorfultag(void) { return colorfultag; }
void srwm_set_colorfultag(int v) { colorfultag = v; }

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
