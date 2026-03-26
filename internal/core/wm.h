#include <X11/XF86keysym.h>
#include <X11/Xatom.h>
#include <X11/Xft/Xft.h>
#include <X11/Xlib.h>
#include <X11/Xproto.h>
#include <X11/Xutil.h>
#include <X11/cursorfont.h>
#include <X11/keysym.h>
#include <errno.h>
#include <limits.h>
#include <locale.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#ifdef XINERAMA
#include <X11/extensions/Xinerama.h>
#endif
#include "drw.h"
#include "util.h"

/* macros */
#define BUTTONMASK (ButtonPressMask | ButtonReleaseMask)
#define CLEANMASK(mask)                                                        \
  (mask & ~(numlockmask | LockMask) &                                          \
   (ShiftMask | ControlMask | Mod1Mask | Mod2Mask | Mod3Mask | Mod4Mask |      \
    Mod5Mask))
#define INTERSECT(x, y, w, h, m)                                               \
  (MAX(0, MIN((x) + (w), (m)->wx + (m)->ww) - MAX((x), (m)->wx)) *             \
   MAX(0, MIN((y) + (h), (m)->wy + (m)->wh) - MAX((y), (m)->wy)))
#define INTERSECTC(x, y, w, h, z)                                              \
  (MAX(0, MIN((x) + (w), (z)->x + (z)->w) - MAX((x), (z)->x)) *                \
   MAX(0, MIN((y) + (h), (z)->y + (z)->h) - MAX((y), (z)->y)))
#define ISVISIBLE(C) ((C)->ws == (C)->mon->current_ws)
#define HIDDEN(C) (!(C)->ismapped)
#define LENGTH(X) (sizeof X / sizeof X[0])
#define MOUSEMASK (BUTTONMASK | PointerMotionMask)
#define WIDTH(X) ((X)->w + 2 * (X)->bw)
#define HEIGHT(X) ((X)->h + 2 * (X)->bw)

extern int ws_count;
#define WS_COUNT (ws_count)
#define TEXTW(X) (drw_fontset_getwidth(drw, (X)) + lrpad)
#define MAXTABS 50

#define SYSTEM_TRAY_REQUEST_DOCK 0

/* XEMBED messages */
#define XEMBED_EMBEDDED_NOTIFY 0
#define XEMBED_WINDOW_ACTIVATE 1
#define XEMBED_FOCUS_IN 4
#define XEMBED_MODALITY_ON 10

#define XEMBED_MAPPED (1 << 0)
#define XEMBED_WINDOW_ACTIVATE 1
#define XEMBED_WINDOW_DEACTIVATE 2

#define VERSION_MAJOR 0
#define VERSION_MINOR 0
#define XEMBED_EMBEDDED_VERSION (VERSION_MAJOR << 16) | VERSION_MINOR

/* enums */
enum { CurNormal, CurResize, CurMove, CurLast }; /* cursor */
enum {
  SchemeNorm,
  SchemeSel,
  SchemeTitle,
  SchemeWs,
  SchemeWs1,
  SchemeWs2,
  SchemeWs3,
  SchemeWs4,
  SchemeWs5,
  SchemeWs6,
  SchemeWs7,
  SchemeWs8,
  SchemeWs9,
  TabSel,
  TabNorm,
  SchemeBtnPrev,
  SchemeBtnNext,
  SchemeBtnClose
}; /* color schemes */
enum {
  NetSupported,
  NetWMName,
  NetWMIcon,
  NetWMState,
  NetWMCheck,
  NetSystemTray,
  NetSystemTrayOP,
  NetSystemTrayOrientation,
  NetSystemTrayOrientationHorz,
  NetWMFullscreen,
  NetActiveWindow,
  NetWMWindowType,
  NetWMWindowTypeDialog,
  NetWMWindowTypeDock,
  NetClientList,
  NetClientInfo,
  NetDesktopNames,
  NetDesktopViewport,
  NetNumberOfDesktops,
  NetCurrentDesktop,
  SrwmCanvasZoom,
  SrwmCanvasCenterX,
  SrwmCanvasCenterY,
  SrwmCanvasActive,
  NetLast
}; /* EWMH atoms */
enum { Manager, Xembed, XembedInfo, XLast }; /* Xembed atoms */
enum {
  WMProtocols,
  WMDelete,
  WMState,
  WMTakeFocus,
  WMLast
}; /* default atoms */
enum {
  ClkWsBar,
  ClkTabBar,
  ClkTabPrev,
  ClkTabNext,
  ClkTabClose,
  ClkStatusText,
  ClkWinTitle,
  ClkClientWin,
  ClkRootWin,
  ClkLast
}; /* clicks */

typedef union {
  int i;
  unsigned int ui;
  float f;
  const void *v;
} Arg;

typedef struct {
  unsigned int click;
  unsigned int mask;
  unsigned int button;
  void (*func)(const Arg *arg);
  const Arg arg;
} Button;

typedef struct Monitor Monitor;
typedef struct {
  int cx, cy;             /* current canvas offset */
  int saved_cx, saved_cy; /* saved offset for layout transitions */
} CanvasOffset;
typedef struct Client Client;
struct Client {
  char name[256];
  int x, y, w, h;
  int oldx, oldy, oldw, oldh;
  int bw, oldbw;
  int ws;              /* workspace index (0-based) */
  int isfullscreen;
  int isurgent;
  int neverfocus;
  int ismapped;        /* tracks whether window is mapped in X11 */
  int saved_cx, saved_cy; /* saved canvas position */
  int saved_cw, saved_ch; /* saved canvas size */
  int was_on_canvas;      /* was this client on the canvas? */
  unsigned int icw, ich;
  Picture icon;
  Client *next;
  Monitor *mon;
  Window win;
};

typedef struct {
  unsigned int mod;
  KeySym keysym;
  void (*func)(const Arg *);
  const Arg arg;
} Key;

typedef struct {
  unsigned int mod;
  KeySym keysym;
  int id;
} DynamicKey;

#define MAX_DYNAMIC_KEYS 256
extern DynamicKey dkeys[MAX_DYNAMIC_KEYS];
extern int dkeys_len;

typedef struct {
  unsigned int click;   // ClkRootWin, ClkClientWin, etc.
  unsigned int mod;     // modifier mask
  unsigned int button;  // Button1-Button5
  int id;               // callback ID for Go
} DynamicButton;

#define MAX_DYNAMIC_BUTTONS 64
extern DynamicButton dbuttons[MAX_DYNAMIC_BUTTONS];
extern int dbuttons_len;

typedef struct Systray Systray;
struct Systray {
  Window win;
  Client *icons;
};

struct Monitor {
  int num;
  int by;             /* bar geometry */
  int ty;             /* tab bar geometry */
  int mx, my, mw, mh; /* screen size */
  int wx, wy, ww, wh; /* window area  */
  int gap;            /* gap value */
  unsigned int borderpx;
  unsigned int colorful_ws;
  CanvasOffset *canvas; /* per-ws canvas offsets, allocated in createmon() */
  float canvas_zoom;      /* global zoom level, shared across all workspaces */
  int showbar;
  int topbar, toptab;
  Client *clients;
  Client *tail; // last client in list, for O(1) append
  Client *sel;
  Monitor *next;
  Monitor *prev;
  Window barwin;
  Window tabwin;
  int ntabs;
  Client *tab_order[MAXTABS];
  int tab_widths[MAXTABS];
  int tab_btn_w[3];
  int current_ws;
  int previous_ws;
  int showbar_per_ws[9];
};

int applysizehints(Client *c, int *x, int *y, int *w, int *h, int interact);
void arrange(Monitor *m);
void arrangemon(Monitor *m);
void attach(Client *c);
void buttonpress(XEvent *e);
void checkotherwm(void);
void cleanup(void);
void cleanupmon(Monitor *mon);
void clientmessage(XEvent *e);
void configure(Client *c);
void configurenotify(XEvent *e);
void configurerequest(XEvent *e);
Monitor *createmon(void);
void destroynotify(XEvent *e);
void detach(Client *c);
Monitor *get_neighbor_monitor(int dir);
void move_window_to_monitor(const Arg *arg);
void drawbar(Monitor *m);
void drawbars(void);
int drawstatusbar(Monitor *m, int bh, char *text);
void drawtab(Monitor *m);
void drawtabs(void);
void enternotify(XEvent *e);
void expose(XEvent *e);
void focus(Client *c);
void focusin(XEvent *e);
void focusstack(const Arg *arg);
void focuswin(const Arg *arg);
void movestack(const Arg *arg);
void shiftview(const Arg *arg);
Atom getatomprop(Client *c, Atom prop);
Picture geticonprop(Window w, unsigned int *icw, unsigned int *ich);
int getrootptr(int *x, int *y);
long getstate(Window w);
unsigned int getsystraywidth(void);
int gettextprop(Window w, Atom atom, char *text, unsigned int size);
void grabbuttons(Client *c, int focused);
void grabkeys(void);
void keypress(XEvent *e);
void killclient(const Arg *arg);
void manage(Window w, XWindowAttributes *wa);
void mappingnotify(XEvent *e);
void maprequest(XEvent *e);
void motionnotify(XEvent *e);
void movemouse(const Arg *arg);
void propertynotify(XEvent *e);
Monitor *recttomon(int x, int y, int w, int h);
void removesystrayicon(Client *i);
void resize(Client *c, int x, int y, int w, int h, int interact);
void resizebarwin(Monitor *m);
void resizeclient(Client *c, int x, int y, int w, int h);
void resizemouse(const Arg *arg);
void resizerequest(XEvent *e);
void restack(Monitor *m);
void run(void);
void scan(void);
int sendevent(Window w, Atom proto, int m, long d0, long d1, long d2, long d3,
              long d4);
void sendmon(Client *c, Monitor *m);
void setclientstate(Client *c, long state);
void setclientwsprop(Client *c);
void setcurrentdesktop(void);
void setdesktopnames(void);
void setfocus(Client *c);
void setfullscreen(Client *c, int fullscreen);
void setnumdesktops(void);
void setup(void);
void setviewport(void);
void seturgent(Client *c, int urg);
void showhide(Monitor *m);
Monitor *systraytomon(Monitor *m);
void move_to_ws(const Arg *arg);
void togglebar(const Arg *arg);
void togglefullscr(const Arg *arg);
void freeicon(Client *c);
void unfocus(Client *c, int setfocus);
void unmanage(Client *c, int destroyed);
void unmapnotify(XEvent *e);
void updatecurrentdesktop(void);
void updatebarpos(Monitor *m);
void updatebars(void);
void updateclientlist(void);
int updategeom(void);
void updatenumlockmask(void);
void rebuild_tab_order(Monitor *m);
void updatestatus(void);
void updatesystray(void);
void updatesystrayicongeom(Client *i, int w, int h);
void updatesystrayiconstate(Client *i, XPropertyEvent *ev);
void updatetitle(Client *c);
void updateicon(Client *c);
void updatewindowtype(Client *c);
void updatewmhints(Client *c);
void view(const Arg *arg);
Client *wintoclient(Window w);
Monitor *wintomon(Window w);
Client *wintosystrayicon(Window w);
int xerror(Display *dpy, XErrorEvent *ee);
int xerrordummy(Display *dpy, XErrorEvent *ee);
int xerrorstart(Display *dpy, XErrorEvent *ee);
void manuallymovecanvas(const Arg *arg);
void centerwindowoncanvas(const Arg *arg);
void homecanvas(const Arg *arg);
void movecanvas(const Arg *arg);
void zoomcanvas(const Arg *arg);
int canvas_edge_autopan(int cursor_x, int cursor_y, Client *exclude, int *pan_dx_out, int *pan_dy_out);
int compositor_running(void);
void publish_canvas_state(Monitor *m);
void window_set_state(Window win, long state);
void window_map(Client *c, int deiconify);
void window_unmap(Window win, int iconify);
extern void srwm_handle_mouse(int id);

extern void (*handler[LASTEvent])(XEvent *);

/* From wm.c */
void win_ht_insert(Window w, Client *c);
void win_ht_remove(Window w);

/* From bar.c */
void ws_to_next(const Arg *arg);
void ws_to_prev(const Arg *arg);
int next_ws(void);
int prev_ws(void);

/* From setup.c */
int isuniquegeom(XineramaScreenInfo *unique, size_t n,
                 XineramaScreenInfo *info);

/* From Go (CGo export) — called in events.c keypress() */
extern void srwm_handle_key(int id);

/* From setup.c / wm.c */
#define ICONSIZE 20
#define ICONSPACING 8
#define MODKEY Mod4Mask
#define ALTKEY Mod1Mask
extern int running;
extern Display *dpy;
extern Systray *systray;
extern const char broken[];
extern char stext[1024];
extern int screen;
extern int sw, sh;
extern int bh;
extern int th;
extern int lrpad;
extern int (*xerrorxlib)(Display *, XErrorEvent *);
extern unsigned int numlockmask;
extern Atom wmatom[], netatom[], xatom[];
extern Cur *cursor[];
extern Clr **scheme, clrborder;
extern Drw *drw;
extern Monitor *mons, *selmon;
extern Window root, wmcheckwin;

/* Config globals */
extern unsigned int borderpx;
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
extern unsigned int ws_underline_padding;
extern unsigned int ws_underline_size;
extern unsigned int ws_underline_offset_from_bar_bottom;
extern int ws_underline_for_all;
extern int toptab;
extern int topbar;
extern int colorful_ws;
extern int ws_colorful_occupied_only;
extern char *ws_labels[];
extern int ws_schemes[];
extern const char *colors[18][3]; // 18 = SchemeBtnClose + 1
extern const char *fonts[1];
extern const Button buttons[];
extern const int buttons_len;

/* Dynamic keys */
extern DynamicKey dkeys[];
extern int dkeys_len;

#define black "#0e0e12"
#define gray2 "#373737"
#define gray3 "#8c8c8c"
#define blue "#6fa6e7"
#define green "#8fb573"
#define red "#c75c6a"
#define orange "#c7a06f"
#define pink "#c56a97"
#define purple "#9a71db"
#define yellow "#c7a06f"
#define white "#ffffff"
