/* bridge.h — Go ↔ C lifecycle interface for srwm */
#ifndef SRWM_BRIDGE_H
#define SRWM_BRIDGE_H

#include <X11/Xlib.h>
#include <X11/keysym.h>

/* Helper to convert string to Keysym */
KeySym srwm_string_to_keysym(const char *name);

/* These are defined in dwm.c (non-static) */
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

/* Called from dwm.c restart() to distinguish restart from quit */
void srwm_request_restart(void);

/* Dynamic keybindings (Go -> C) */
void srwm_add_keybinding(unsigned int mod, KeySym keysym, int id);
void srwm_clear_keybindings(void);
void srwm_grabkeys(void);

/* Config getters and setters */
int srwm_get_layout_mode(void);  
void srwm_set_layout_mode(int);
unsigned int srwm_get_borderpx(void);
void srwm_set_borderpx(unsigned int);
unsigned int srwm_get_px_till_snapping_to_screen_edge(void);
void srwm_set_px_till_snapping_to_screen_edge(unsigned int);
unsigned int srwm_get_gaps(void);
void srwm_set_gaps(unsigned int);
unsigned int srwm_get_systraypinning(void);
void srwm_set_systraypinning(unsigned int);
unsigned int srwm_get_systrayspacing(void);
void srwm_set_systrayspacing(unsigned int);
int srwm_get_systray_enable(void);
void srwm_set_systray_enable(int);
int srwm_get_showbar(void);
void srwm_set_showbar(int);
int srwm_get_bar_horizontal_padding(void);
void srwm_set_bar_horizontal_padding(int);
int srwm_get_bar_vertical_padding(void);
void srwm_set_bar_vertical_padding(int);
int srwm_get_tab_height(void);
void srwm_set_tab_height(int);
int srwm_get_tab_tile_vertical_padding(void);  
void srwm_set_tab_tile_vertical_padding(int);
int srwm_get_tab_tile_inner_padding_horizontal(void);
void srwm_set_tab_tile_inner_padding_horizontal(int);
int srwm_get_tab_tile_outer_padding_horizontal(void);
void srwm_set_tab_tile_outer_padding_horizontal(int);
unsigned int srwm_get_tag_underline_padding(void);
void srwm_set_tag_underline_padding(unsigned int);
unsigned int srwm_get_tag_underline_size(void);
void srwm_set_tag_underline_size(unsigned int);
unsigned int srwm_get_tag_underline_offset_from_bar_bottom(void);
void srwm_set_tag_underline_offset_from_bar_bottom(unsigned int);
int srwm_get_tag_underline_for_all_tags(void);
void srwm_set_tag_underline_for_all_tags(int);
int srwm_get_toptab(void);
void srwm_set_toptab(int);
int srwm_get_topbar(void);
void srwm_set_topbar(int);
void srwm_set_font(const char* font);
void srwm_set_color(int scheme, int slot, const char* hex);
void srwm_set_tag(int idx, const char* name);
void srwm_set_tags_len(int len);
void srwm_set_tag_colorful_occupied_only(int val);
void srwm_set_tagscheme(int idx, int scheme_idx);

/* Actions (Go -> C wrappers for internal static functions) */
void srwm_action_killclient(void);
void srwm_action_togglefloating(void);
void srwm_action_togglefullscr(void);
void srwm_action_focusstack(int dir);
void srwm_action_shiftview(int dir);
void srwm_action_tagtoprev(void);
void srwm_action_tagtonext(void);
void srwm_action_move_tag_to_monitor(int dir);
void srwm_action_view(unsigned int mask);
void srwm_action_toggleview(unsigned int mask);
void srwm_action_tag(unsigned int mask);
void srwm_action_toggletag(unsigned int mask);

/* Canvas mode actions */  
void srwm_action_movecanvas(int dir);  
void srwm_action_homecanvas(void);  
void srwm_action_centerwindowoncanvas(void);  
void srwm_action_manuallymovecanvas(void);
void srwm_action_zoomcanvas(int dir);

/* Called from C -> Go when a dynamic key is pressed */
extern void srwm_handle_key(int id);

#endif /* SRWM_BRIDGE_H */
