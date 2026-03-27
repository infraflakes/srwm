/* bridge.h — Go ↔ C lifecycle interface for srwm */
#ifndef SRWM_BRIDGE_H
#define SRWM_BRIDGE_H

#include <X11/Xlib.h>
#include <X11/keysym.h>

/* Helper to convert string to Keysym */
KeySym srwm_string_to_keysym(const char *name);

/* These are defined in srwm.c (non-static) */
extern int running;
extern Display* dpy;

extern void checkotherwm(void);
extern void setup(void);
extern void scan(void);
extern void run(void);
extern void cleanup(void);

/* Bridge lifecycle — called by Go */
int  srwm_init_display(void);
void srwm_init_setup(void);
void srwm_run(void);
void srwm_cleanup(void);
void srwm_quit(void);
int  srwm_should_restart(void);

/* Status bar — Go sets the root window name */
void srwm_set_status(const char *text);

/* Called from srwm.c restart() to distinguish restart from quit */
void srwm_request_restart(void);

/* Dynamic keybindings (Go -> C) */
void srwm_add_keybinding(unsigned int mod, KeySym keysym, int id);
void srwm_clear_keybindings(void);
void srwm_grabkeys(void);
/* Dynamic mouse bindings (Go -> C) */
void srwm_add_mousebinding(unsigned int click, unsigned int mod, unsigned int button, int id);
void srwm_clear_mousebindings(void);

/* Called from C -> Go when a dynamic mouse button is pressed */
extern void srwm_handle_mouse(int id);

/* Config getters and setters */
unsigned int srwm_get_borderpx(void);
void srwm_set_borderpx(unsigned int);
int srwm_get_tab_height(void);
void srwm_set_tab_height(int);
int srwm_get_tab_tile_vertical_padding(void);  
void srwm_set_tab_tile_vertical_padding(int);
int srwm_get_tab_tile_inner_padding_horizontal(void);
void srwm_set_tab_tile_inner_padding_horizontal(int);
int srwm_get_tab_tile_outer_padding_horizontal(void);
void srwm_set_tab_tile_outer_padding_horizontal(int);
int srwm_get_toptab(void);
void srwm_set_toptab(int);
void srwm_set_font(const char* font);
void srwm_set_color(int scheme, int slot, const char* hex);

/* Dedicated color setters */
void srwm_set_border_active(const char* hex);
void srwm_set_border_inactive(const char* hex);
void srwm_set_ws_label(int idx, const char* name);
void srwm_set_ws_count(int len);
void srwm_set_ws_scheme(int idx, int scheme_idx);

/* Actions (Go -> C wrappers for internal static functions) */
void srwm_action_killclient(void);
void srwm_action_togglefullscr(void);
void srwm_action_focusstack(int dir);
void srwm_action_shiftview(int dir);
void srwm_action_ws_to_prev(void);
void srwm_action_ws_to_next(void);
void srwm_action_move_window_to_monitor(int dir);
void srwm_action_view(int ws);
void srwm_action_move_to_ws(int ws);

/* Canvas mode actions */  
void srwm_action_movecanvas(int dir);  
void srwm_action_homecanvas(void);  
void srwm_action_centerwindowoncanvas(void);  
void srwm_action_zoomcanvas(int dir);

/* Called from C -> Go when a dynamic key is pressed */
extern void srwm_handle_key(int id);

#endif /* SRWM_BRIDGE_H */
