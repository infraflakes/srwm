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
int srwm_get_tab_vertical_padding(void);
void srwm_set_tab_vertical_padding(int);
int srwm_get_tab_in_horizontal_padding(void);
void srwm_set_tab_in_horizontal_padding(int);
int srwm_get_tab_out_horizontal_padding(void);
void srwm_set_tab_out_horizontal_padding(int);
int srwm_get_tag_preview_size(void);
void srwm_set_tag_preview_size(int);
int srwm_get_tag_preview_enable(void);
void srwm_set_tag_preview_enable(int);
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
int srwm_get_new_window_appear_on_end(void);
void srwm_set_new_window_appear_on_end(int);
int srwm_get_colorfultag(void);
void srwm_set_colorfultag(int);
void srwm_set_font(const char* font);
void srwm_set_color(int scheme, int slot, const char* hex);
void srwm_set_tag(int idx, const char* name);
void srwm_set_tags_len(int len);
void srwm_set_tagscheme(int idx, int scheme_idx);

/* Called from C -> Go when a dynamic key is pressed */
extern void srwm_handle_key(int id);

#endif /* SRWM_BRIDGE_H */
