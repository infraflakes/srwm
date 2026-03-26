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

#include <fontconfig/fontconfig.h>
#include <sys/stat.h>
  
static void ensure_fontconfig(void) {
  /* If FONTCONFIG_FILE is already set, trust the user/wrapper */
  if (getenv("FONTCONFIG_FILE")) return;
  
  /* Common system fontconfig paths */
  static const char *candidates[] = {
    "/etc/fonts/fonts.conf",
    "/usr/share/fontconfig/fonts.conf",
    "/usr/local/etc/fonts/fonts.conf",
    NULL
  };

  /* Check if fontconfig can find its config without help */
  if (FcConfigGetCurrent()) return;

  struct stat st;
  for (const char **p = candidates; *p; p++) {
    if (stat(*p, &st) == 0) {
      setenv("FONTCONFIG_FILE", *p, 0);
      FcInitReinitialize();
      return;
    }
  }
  /* If nothing found, fontconfig will use its compiled-in default (may fail) */
}

int srwm_init_display(void) {
  /* Reset for re-init on restart */
  running = 1;
  restart_requested = 0;
  ensure_fontconfig();

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

/* Dynamic keys defined in srwm.c */
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
extern void add_dynamic_button(unsigned int click, unsigned int mod, unsigned int button, int id);
extern void clear_dynamic_buttons(void);

void srwm_add_mousebinding(unsigned int click, unsigned int mod, unsigned int button, int id) {
  add_dynamic_button(click, mod, button, id);
}

void srwm_clear_mousebindings(void) {
  clear_dynamic_buttons();
}

unsigned int srwm_get_borderpx(void) { return borderpx; }
void srwm_set_borderpx(unsigned int v) { borderpx = v; }

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

unsigned int srwm_get_ws_underline_padding(void) { return ws_underline_padding; }
void srwm_set_ws_underline_padding(unsigned int v) { ws_underline_padding = v; }

unsigned int srwm_get_ws_underline_size(void) { return ws_underline_size; }
void srwm_set_ws_underline_size(unsigned int v) { ws_underline_size = v; }

unsigned int srwm_get_ws_underline_offset_from_bar_bottom(void) { return ws_underline_offset_from_bar_bottom; }
void srwm_set_ws_underline_offset_from_bar_bottom(unsigned int v) { ws_underline_offset_from_bar_bottom = v; }

int srwm_get_ws_underline_for_all(void) { return ws_underline_for_all; }
void srwm_set_ws_underline_for_all(int v) { ws_underline_for_all = v; }

int srwm_get_toptab(void) { return toptab; }
void srwm_set_toptab(int v) { toptab = v; }

int srwm_get_topbar(void) { return topbar; }
void srwm_set_topbar(int v) { topbar = v; }

/* Dedicated color getters and setters */
void srwm_set_border_active(const char* hex) {
    drw_clr_create(drw, &border_active, hex);
}
void srwm_set_border_inactive(const char* hex) {
    drw_clr_create(drw, &border_inactive, hex);
}
void srwm_set_bar_bg(const char* hex) {
    drw_clr_create(drw, &bar_bg, hex);
}

/* Font and colors */
extern const char* fonts[];
void srwm_set_font(const char* font) { fonts[0] = font; }

extern const char* colors[][3];
static int colors_owned[18][3] = {0}; /* tracks which slots were strdup'd */

void srwm_set_color(int scheme, int slot, const char* hex) {
   if (slot < 0 || slot >= 3) return;
   if (colors_owned[scheme][slot])
     free((void*)colors[scheme][slot]);
   colors[scheme][slot] = strdup(hex);
   colors_owned[scheme][slot] = 1;
}

extern char* ws_labels[];
extern int ws_count;
static int ws_labels_owned[9] = {0};

void srwm_set_ws_label(int idx, const char* name) {
  if (idx < 0 || idx >= 9) return;
  if (ws_labels_owned[idx])
    free(ws_labels[idx]);
  ws_labels[idx] = strdup(name);
  ws_labels_owned[idx] = 1;
}

void srwm_set_ws_count(int len) {
  if (len >= 0 && len <= 9) ws_count = len;
}

extern int ws_schemes[];
void srwm_set_ws_scheme(int idx, int scheme_idx) {
  if (idx >= 0 && idx < 9) ws_schemes[idx] = scheme_idx;
}

/* srwm_action bindings mapping primitives to internal Arg structs */
void srwm_action_killclient(void) { killclient(&(Arg){0}); }
void srwm_action_togglefullscr(void) { togglefullscr(&(Arg){0}); }
void srwm_action_focusstack(int dir) { focusstack(&(Arg){.i = dir}); centerwindowoncanvas(&(Arg){0}); }
void srwm_action_shiftview(int dir) { shiftview(&(Arg){.i = dir}); }
void srwm_action_ws_to_prev(void) { ws_to_prev(&(Arg){0}); }
void srwm_action_ws_to_next(void) { ws_to_next(&(Arg){0}); }
void srwm_action_move_window_to_monitor(int dir) { move_window_to_monitor(&(Arg){.i = dir}); }
void srwm_action_view(int ws) { view(&(Arg){.i = ws}); }
void srwm_action_move_to_ws(int ws) { move_to_ws(&(Arg){.i = ws}); }
void srwm_set_ws_colorful_occupied_only(int val) {ws_colorful_occupied_only = val;}

void srwm_action_movecanvas(int dir) { movecanvas(&(Arg){.i = dir}); }  
void srwm_action_homecanvas(void) { homecanvas(&(Arg){0}); }  
void srwm_action_centerwindowoncanvas(void) { centerwindowoncanvas(&(Arg){0}); }  
void srwm_action_zoomcanvas(int dir) { zoomcanvas(&(Arg){.i = dir}); }
