#include "wm.h"
#include <sys/select.h>

int screen;  
int sw, sh;  
int bh;  
int th = 0;  
int lrpad;  
int (*xerrorxlib)(Display*, XErrorEvent*);  
unsigned int numlockmask = 0;  
Atom wmatom[WMLast], netatom[NetLast], xatom[XLast];  
int running = 1;  
Cur* cursor[CurLast];  
Clr **scheme;
Display* dpy;  
Drw* drw;  
Monitor *mons, *selmon;  
Window root, wmcheckwin;  
DynamicKey dkeys[MAX_DYNAMIC_KEYS];  
int dkeys_len = 0;
DynamicButton dbuttons[MAX_DYNAMIC_BUTTONS];
int dbuttons_len = 0;

void checkotherwm(void) {
  xerrorxlib = XSetErrorHandler(xerrorstart);
  /* this causes an error if some other window manager is running */
  XSelectInput(dpy, DefaultRootWindow(dpy), SubstructureRedirectMask);
  XSync(dpy, False);
  XSetErrorHandler(xerror);
  XSync(dpy, False);
}

void cleanup(void) {
  Arg a = {.ui = ~0};
  Monitor* m;
  size_t i;

  view(&a);
  for (m = mons; m; m = m->next)
    while (m->stack) unmanage(m->stack, 0);
  XUngrabKey(dpy, AnyKey, AnyModifier, root);
  while (mons) cleanupmon(mons);
  if (systray_enable) {
    XUnmapWindow(dpy, systray->win);
    XDestroyWindow(dpy, systray->win);
    free(systray);
    systray = NULL;
  }
  for (i = 0; i < CurLast; i++) drw_cur_free(drw, cursor[i]);
  for (i = 0; i < LENGTH(colors) + 1; i++) free(scheme[i]);
  free(scheme);
  scheme = NULL;
  XDestroyWindow(dpy, wmcheckwin);
  wmcheckwin = None;
  drw_free(drw);
  drw = NULL;
  mons = selmon = NULL;
  XSync(dpy, False);
  XSetInputFocus(dpy, PointerRoot, RevertToPointerRoot, CurrentTime);
  XDeleteProperty(dpy, root, netatom[NetActiveWindow]);
}

void cleanupmon(Monitor* mon) {
  Monitor* m;

  if (mon == mons) {
    mons = mons->next;
    if (mons) mons->prev = NULL;
  } else {
    for (m = mons; m && m->next != mon; m = m->next);
    m->next = mon->next;
    if (m->next) m->next->prev = m;
  }
  XUnmapWindow(dpy, mon->barwin);
  XDestroyWindow(dpy, mon->barwin);
  if (mon->tabwin) {
  XUnmapWindow(dpy, mon->tabwin);
  XDestroyWindow(dpy, mon->tabwin);
  }
  free(mon->canvas);
  free(mon);
}

Monitor* createmon(void) {
  Monitor* m;

  m = ecalloc(1, sizeof(Monitor));
  m->tagset[0] = m->tagset[1] = 1;
  m->showbar = showbar;
  m->topbar = topbar;
  m->toptab = toptab;
  m->ntabs = 0;
  m->colorfultag = colorfultag ? colorfultag : 0;
  m->gap = gaps;
  m->borderpx = borderpx;
  m->prev = NULL;
  m->curtag = m->prevtag = 1;
  m->showbar_mask = showbar ? ~0u : 0u;
  m->canvas_mode = layout_mode;
  m->canvas = ecalloc(9, sizeof(CanvasOffset)); /* one per tag, max 9 */
  for (int i = 0; i < 9; i++) {
    m->canvas[i].zoom = 1.0f;
  }

  return m;
}

void setup(void) {
  int i;
  XSetWindowAttributes wa;
  Atom utf8string;

  /* init screen */
  screen = DefaultScreen(dpy);
  sw = DisplayWidth(dpy, screen);
  sh = DisplayHeight(dpy, screen);
  root = RootWindow(dpy, screen);
  drw = drw_create(dpy, screen, root, sw, sh);
  if (!drw_fontset_create(drw, fonts, LENGTH(fonts)))
    die("no fonts could be loaded.");
  lrpad = drw->fonts->h;
  bh = drw->fonts->h + 2 + bar_vertical_padding;
  th = tab_height;
  // bh_n = tab_height;
  updategeom();
  /* init atoms */
  utf8string = XInternAtom(dpy, "UTF8_STRING", False);
  wmatom[WMProtocols] = XInternAtom(dpy, "WM_PROTOCOLS", False);
  wmatom[WMDelete] = XInternAtom(dpy, "WM_DELETE_WINDOW", False);
  wmatom[WMState] = XInternAtom(dpy, "WM_STATE", False);
  wmatom[WMTakeFocus] = XInternAtom(dpy, "WM_TAKE_FOCUS", False);
  netatom[NetActiveWindow] = XInternAtom(dpy, "_NET_ACTIVE_WINDOW", False);
  netatom[NetSupported] = XInternAtom(dpy, "_NET_SUPPORTED", False);
  netatom[NetSystemTray] = XInternAtom(dpy, "_NET_SYSTEM_TRAY_S0", False);
  netatom[NetSystemTrayOP] = XInternAtom(dpy, "_NET_SYSTEM_TRAY_OPCODE", False);
  netatom[NetSystemTrayOrientation] =
      XInternAtom(dpy, "_NET_SYSTEM_TRAY_ORIENTATION", False);
  netatom[NetSystemTrayOrientationHorz] =
      XInternAtom(dpy, "_NET_SYSTEM_TRAY_ORIENTATION_HORZ", False);
  netatom[NetWMName] = XInternAtom(dpy, "_NET_WM_NAME", False);
  netatom[NetWMIcon] = XInternAtom(dpy, "_NET_WM_ICON", False);
  netatom[NetWMState] = XInternAtom(dpy, "_NET_WM_STATE", False);
  netatom[NetWMCheck] = XInternAtom(dpy, "_NET_SUPPORTING_WM_CHECK", False);
  netatom[NetWMFullscreen] =
      XInternAtom(dpy, "_NET_WM_STATE_FULLSCREEN", False);
  netatom[NetWMWindowType] = XInternAtom(dpy, "_NET_WM_WINDOW_TYPE", False);
  netatom[NetWMWindowTypeDialog] =
      XInternAtom(dpy, "_NET_WM_WINDOW_TYPE_DIALOG", False);
  netatom[NetWMWindowTypeDock] =
    XInternAtom(dpy, "_NET_WM_WINDOW_TYPE_DOCK", False);
  netatom[NetClientList] = XInternAtom(dpy, "_NET_CLIENT_LIST", False);
  xatom[Manager] = XInternAtom(dpy, "MANAGER", False);
  xatom[Xembed] = XInternAtom(dpy, "_XEMBED", False);
  xatom[XembedInfo] = XInternAtom(dpy, "_XEMBED_INFO", False);
  netatom[NetDesktopViewport] =
      XInternAtom(dpy, "_NET_DESKTOP_VIEWPORT", False);
  netatom[NetNumberOfDesktops] =
      XInternAtom(dpy, "_NET_NUMBER_OF_DESKTOPS", False);
  netatom[NetCurrentDesktop] = XInternAtom(dpy, "_NET_CURRENT_DESKTOP", False);
  netatom[NetDesktopNames] = XInternAtom(dpy, "_NET_DESKTOP_NAMES", False);
  netatom[NetClientInfo] = XInternAtom(dpy, "_NET_CLIENT_INFO", False);
  netatom[SrwmCanvasZoom] = XInternAtom(dpy, "_SRWM_CANVAS_ZOOM", False);
  netatom[SrwmCanvasCenterX] = XInternAtom(dpy, "_SRWM_CANVAS_CENTER_X", False);
  netatom[SrwmCanvasCenterY] = XInternAtom(dpy, "_SRWM_CANVAS_CENTER_Y", False);
  netatom[SrwmCanvasActive] = XInternAtom(dpy, "_SRWM_CANVAS_ACTIVE", False);
  /* init cursors */
  cursor[CurNormal] = drw_cur_create(drw, XC_left_ptr);
  cursor[CurResize] = drw_cur_create(drw, XC_sizing);
  cursor[CurMove] = drw_cur_create(drw, XC_fleur);
  /* init appearance */
  scheme = ecalloc(LENGTH(colors) + 1, sizeof(Clr*));
  scheme[LENGTH(colors)] = drw_scm_create(drw, colors[0], 3);
  for (i = 0; i < LENGTH(colors); i++)
    scheme[i] = drw_scm_create(drw, colors[i], 3);
  /* init system tray */
  updatesystray();
  /* init bars */
  updatebars();
  updatestatus();
  updatebarpos(selmon);
  /* supporting window for NetWMCheck */
  wmcheckwin = XCreateSimpleWindow(dpy, root, 0, 0, 1, 1, 0, 0, 0);
  XChangeProperty(dpy, wmcheckwin, netatom[NetWMCheck], XA_WINDOW, 32,
                  PropModeReplace, (unsigned char*)&wmcheckwin, 1);
  XChangeProperty(dpy, wmcheckwin, netatom[NetWMName], utf8string, 8,
                  PropModeReplace, (unsigned char*)"srwm", 4);
  XChangeProperty(dpy, root, netatom[NetWMCheck], XA_WINDOW, 32,
                  PropModeReplace, (unsigned char*)&wmcheckwin, 1);
  /* EWMH support per view */
  XChangeProperty(dpy, root, netatom[NetSupported], XA_ATOM, 32,
                  PropModeReplace, (unsigned char*)netatom, NetLast);
  setnumdesktops();
  setcurrentdesktop();
  setdesktopnames();
  setviewport();
  XDeleteProperty(dpy, root, netatom[NetClientList]);
  XDeleteProperty(dpy, root, netatom[NetClientInfo]);
  /* select events */
  wa.cursor = cursor[CurNormal]->cursor;
  wa.event_mask = SubstructureRedirectMask | SubstructureNotifyMask |
                  ButtonPressMask | PointerMotionMask | EnterWindowMask |
                  LeaveWindowMask | StructureNotifyMask | PropertyChangeMask;
  XChangeWindowAttributes(dpy, root, CWEventMask | CWCursor, &wa);
  XSelectInput(dpy, root, wa.event_mask);
  grabkeys();
  focus(NULL);
  publish_canvas_state(selmon);
}

void run(void) {
  XEvent ev;
  int x11_fd = ConnectionNumber(dpy);
  XSync(dpy, False);
  while (running) {
    // Check if we need continuous auto-pan
    int need_autopan = selmon && selmon->canvas_mode &&
                       selmon->canvas[getcurrenttag(selmon)].zoom < 1.0f;

    if (need_autopan && !XPending(dpy)) {
      fd_set fds;
      FD_ZERO(&fds);
      FD_SET(x11_fd, &fds);
      struct timeval tv = { .tv_sec = 0, .tv_usec = 16000 }; // 16ms = ~60fps
      int ret = select(x11_fd + 1, &fds, NULL, NULL, &tv);

      if (ret == 0) {
        // Timeout — check if cursor is at edge and auto-pan
        int cx, cy;
        Window dummy_w;
        int di;
        unsigned int dui;
        if (XQueryPointer(dpy, root, &dummy_w, &dummy_w, &cx, &cy, &di, &di, &dui)) {
          canvas_edge_autopan(cx, cy, NULL, NULL, NULL);
        }
        continue;
      }
      // If ret > 0, fall through to process the X event
    }

    if (XNextEvent(dpy, &ev))
      break;
    if (handler[ev.type])
      handler[ev.type](&ev);
  }
}

void scan(void) {
  unsigned int i, num;
  Window d1, d2, *wins = NULL;
  XWindowAttributes wa;

  if (XQueryTree(dpy, root, &d1, &d2, &wins, &num)) {
    for (i = 0; i < num; i++) {
      if (!XGetWindowAttributes(dpy, wins[i], &wa) || wa.override_redirect ||
          XGetTransientForHint(dpy, wins[i], &d1))
        continue;
      if (wa.map_state == IsViewable || getstate(wins[i]) == IconicState)
        manage(wins[i], &wa);
    }
    for (i = 0; i < num; i++) { /* now the transients */
      if (!XGetWindowAttributes(dpy, wins[i], &wa)) continue;
      if (XGetTransientForHint(dpy, wins[i], &d1) &&
          (wa.map_state == IsViewable || getstate(wins[i]) == IconicState))
        manage(wins[i], &wa);
    }
    if (wins) XFree(wins);
  }
}

int updategeom(void) {
  int dirty = 0;

#ifdef XINERAMA
  if (XineramaIsActive(dpy)) {
    int i, j, n, nn;
    Client* c;
    Monitor* m;
    XineramaScreenInfo* info = XineramaQueryScreens(dpy, &nn);
    XineramaScreenInfo* unique = NULL;

    for (n = 0, m = mons; m; m = m->next, n++);
    /* only consider unique geometries as separate screens */
    unique = ecalloc(nn, sizeof(XineramaScreenInfo));
    for (i = 0, j = 0; i < nn; i++)
      if (isuniquegeom(unique, j, &info[i]))
        memcpy(&unique[j++], &info[i], sizeof(XineramaScreenInfo));
    XFree(info);
    nn = j;
    /* new monitors if nn > n */
    for (i = n; i < nn; i++) {
      for (m = mons; m && m->next; m = m->next);
      if (m) {
        m->next = createmon();
        m->next->prev = m;
      } else {
        mons = createmon();
      }
    }
    for (i = 0, m = mons; i < nn && m; m = m->next, i++)
      if (i >= n || unique[i].x_org != m->mx || unique[i].y_org != m->my ||
          unique[i].width != m->mw || unique[i].height != m->mh) {
        dirty = 1;
        m->num = i;
        m->mx = m->wx = unique[i].x_org;
        m->my = m->wy = unique[i].y_org;
        m->mw = m->ww = unique[i].width;
        m->mh = m->wh = unique[i].height;
        updatebarpos(m);
      }
    /* removed monitors if n > nn */
    for (i = nn; i < n; i++) {
      for (m = mons; m && m->next; m = m->next);
      while ((c = m->clients)) {
        dirty = 1;
        m->clients = c->next;
        detachstack(c);
        c->mon = mons;
        attach(c);
        attachstack(c);
      }
      if (m == selmon) selmon = mons;
      cleanupmon(m);
    }
    free(unique);
  } else
#endif /* XINERAMA */
  {    /* default monitor setup */
    if (!mons) mons = createmon();
    if (mons->mw != sw || mons->mh != sh) {
      dirty = 1;
      mons->mw = mons->ww = sw;
      mons->mh = mons->wh = sh;
      updatebarpos(mons);
    }
  }
  if (dirty) {
    selmon = mons;
    selmon = wintomon(root);
  }
  return dirty;
}

void updatenumlockmask(void) {
  unsigned int i, j;
  XModifierKeymap* modmap;

  numlockmask = 0;
  modmap = XGetModifierMapping(dpy);
  for (i = 0; i < 8; i++)
    for (j = 0; j < modmap->max_keypermod; j++)
      if (modmap->modifiermap[i * modmap->max_keypermod + j] ==
          XKeysymToKeycode(dpy, XK_Num_Lock))
        numlockmask = (1 << i);
  XFreeModifiermap(modmap);
}

void setcurrentdesktop(void) {
  long data[] = {0};
  XChangeProperty(dpy, root, netatom[NetCurrentDesktop], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 1);
}
void setdesktopnames(void) {
  XTextProperty text;
  if (Xutf8TextListToTextProperty(dpy, tags, TAGSLENGTH, XUTF8StringStyle, &text) != Success) {
    /* Fallback for static musl builds where locale/Xutf8 support is unavailable */
    if (!XStringListToTextProperty(tags, TAGSLENGTH, &text))
      return;
  }
  XSetTextProperty(dpy, root, &text, netatom[NetDesktopNames]);
  XFree(text.value);
}

void setnumdesktops(void) {
  long data[] = {TAGSLENGTH};
  XChangeProperty(dpy, root, netatom[NetNumberOfDesktops], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 1);
}

void setviewport(void) {
  long data[] = {0, 0};
  XChangeProperty(dpy, root, netatom[NetDesktopViewport], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 2);
}

int sendevent(Window w, Atom proto, int mask, long d0, long d1, long d2, long d3, long d4) {
  int n;
  Atom *protocols, mt;
  int exists = 0;
  XEvent ev;

  if (proto == wmatom[WMTakeFocus] || proto == wmatom[WMDelete]) {
    mt = wmatom[WMProtocols];
    if (XGetWMProtocols(dpy, w, &protocols, &n)) {
      while (!exists && n--) exists = protocols[n] == proto;
      XFree(protocols);
    }
  } else {
    exists = True;
    mt = proto;
  }
  if (exists) {
    ev.type = ClientMessage;
    ev.xclient.window = w;
    ev.xclient.message_type = mt;
    ev.xclient.format = 32;
    ev.xclient.data.l[0] = d0;
    ev.xclient.data.l[1] = d1;
    ev.xclient.data.l[2] = d2;
    ev.xclient.data.l[3] = d3;
    ev.xclient.data.l[4] = d4;
    XSendEvent(dpy, w, False, mask, &ev);
  }
  return exists;
}

/* There's no way to check accesses to destroyed windows, thus those cases are
 * ignored (especially on UnmapNotify's). Other types of errors call Xlibs
 * default error handler, which may call exit. */
int xerror(Display* dpy, XErrorEvent* ee) {
  if (ee->error_code == BadWindow || ee->error_code == BadLength ||  /* oversized RENDER glyph requests (e.g. Nerd Fonts on XLibre) */
      (ee->request_code == X_SetInputFocus && ee->error_code == BadMatch) ||
      (ee->request_code == X_PolyText8 && ee->error_code == BadDrawable) ||
      (ee->request_code == X_PolyFillRectangle &&
       ee->error_code == BadDrawable) ||
      (ee->request_code == X_PolySegment && ee->error_code == BadDrawable) ||
      (ee->request_code == X_ConfigureWindow && ee->error_code == BadMatch) ||
      (ee->request_code == X_GrabButton && ee->error_code == BadAccess) ||
      (ee->request_code == X_GrabKey && ee->error_code == BadAccess) ||
      (ee->request_code == X_CopyArea && ee->error_code == BadDrawable))
    return 0;
  fprintf(stderr, "srwm: fatal error: request code=%d, error code=%d\n",
          ee->request_code, ee->error_code);
  return xerrorxlib(dpy, ee); /* may call exit */
}

int xerrordummy(Display* dpy, XErrorEvent* ee) { return 0; }

/* Startup Error handler to check if another window manager
 * is already running. */
int xerrorstart(Display* dpy, XErrorEvent* ee) {
  die("srwm: another window manager is already running");
  return -1;
}

void grabkeys(void) {
  updatenumlockmask();
  {
    unsigned int i, j, k;
    unsigned int modifiers[] = {0, LockMask, numlockmask,
                                numlockmask | LockMask};
    int start, end, skip;
    KeySym* syms;
    XUngrabKey(dpy, AnyKey, AnyModifier, root);
    XDisplayKeycodes(dpy, &start, &end);
    syms = XGetKeyboardMapping(dpy, start, end - start + 1, &skip);
    if (!syms) return;
    for (k = start; k <= end; k++) {
      for (i = 0; i < dkeys_len; i++) {
        if (dkeys[i].keysym == syms[(k - start) * skip]) {
          for (j = 0; j < LENGTH(modifiers); j++) {
            XGrabKey(dpy, k, dkeys[i].mod | modifiers[j], root, True,
                     GrabModeAsync, GrabModeAsync);
          }
        }
      }
    }
    XFree(syms);
  }
}

void add_dynamic_key(unsigned int mod, KeySym keysym, int id) {
  if (dkeys_len < MAX_DYNAMIC_KEYS) {
    dkeys[dkeys_len].mod = mod;
    dkeys[dkeys_len].keysym = keysym;
    dkeys[dkeys_len].id = id;
    dkeys_len++;
  }
}

void clear_dynamic_keys(void) {
  dkeys_len = 0;
}

void add_dynamic_button(unsigned int click, unsigned int mod, unsigned int button, int id) {
  if (dbuttons_len < MAX_DYNAMIC_BUTTONS) {
    dbuttons[dbuttons_len].click = click;
    dbuttons[dbuttons_len].mod = mod;
    dbuttons[dbuttons_len].button = button;
    dbuttons[dbuttons_len].id = id;
    dbuttons_len++;
  }
}

void clear_dynamic_buttons(void) {
  dbuttons_len = 0;
}

int isuniquegeom(XineramaScreenInfo* unique, size_t n,
                        XineramaScreenInfo* info) {
  while (n--)
    if (unique[n].x_org == info->x_org && unique[n].y_org == info->y_org &&
        unique[n].width == info->width && unique[n].height == info->height)
      return 0;
  return 1;
}

/* Default configuration fallback values - can be overridden via Lua */
unsigned int borderpx = 0;
unsigned int gaps = 0;
unsigned int systraypinning = 0;
unsigned int systrayspacing = 2;
int systray_enable = 1;
int showbar = 1;
int bar_horizontal_padding = 10;
int bar_vertical_padding = 0;
int tab_height = 35;
int tab_tile_vertical_padding = 5;
int tab_tile_inner_padding_horizontal = 15;
int tab_tile_outer_padding_horizontal = 15;
unsigned int tag_underline_padding = 5;
unsigned int tag_underline_size = 2;
unsigned int tag_underline_offset_from_bar_bottom = 0;
int tag_underline_for_all_tags = 0;
int toptab = 1;
int topbar = 1;
int colorfultag = 1;
int tag_colorful_occupied_only = 1;
int layout_mode = 0;  /* 0 = monocle (default), 1 = canvas */
const char* fonts[] = {"JetBrainsMonoNerdFont:size=13"};
const char* colors[][3] = {
    [SchemeNorm] = {gray3, black, gray2},
    [SchemeSel] = {gray3, blue, blue},
    [SchemeTitle] = {white, black, black},
    [TabSel] = {black, purple, black},
    [TabNorm] = {gray3, black, black},
    [SchemeTag] = {gray2, black, black},
    [SchemeTag1] = {blue, black, black},
    [SchemeTag2] = {purple, black, black},
    [SchemeTag3] = {pink, black, black},
    [SchemeTag4] = {blue, black, black},
    [SchemeTag5] = {purple, black, black},
    [SchemeTag6] = {pink, black, black},
    [SchemeTag7] = {blue, black, black},
    [SchemeTag8] = {purple, black, black},
    [SchemeTag9] = {pink, black, black},
    [SchemeBtnPrev] = {green, black, black},
    [SchemeBtnNext] = {yellow, black, black},
    [SchemeBtnClose] = {red, black, black},
};
char* tags[9] = {"1", "2", "3", "4", "5", "6", "7", "8", "9"};
int tags_len = 5;
int tagschemes[9] = {SchemeTag1, SchemeTag2, SchemeTag3,
                     SchemeTag4, SchemeTag5, SchemeTag6,
                     SchemeTag7, SchemeTag8, SchemeTag9};

const Rule rules[] = {
    { "Toolkit",  NULL,       "Picture-in-Picture",   0,         1,          -1 },
    { "firefox",  NULL,       "Picture-in-Picture",   0,         1,          -1 },
    { "Chromium", NULL,       "Picture-in-Picture",   0,         1,          -1 },
};

/* button definitions */
const Button buttons[] = {
    {ClkRootWin, MODKEY, Button1, manuallymovecanvas, {0}},  // drag canvas on blank desktop
    {ClkClientWin, MODKEY, Button1, moveorplace, {.i = 0}},
    {ClkClientWin, MODKEY, Button2, togglefloating, {0}},
    {ClkClientWin, MODKEY, Button3, resizemouse, {0}},
    {ClkTagBar, 0, Button1, view, {0}},
    {ClkTagBar, 0, Button3, toggleview, {0}},
    {ClkTagBar, MODKEY, Button1, tag, {0}},
    {ClkTagBar, MODKEY, Button3, toggletag, {0}},
    {ClkTabBar, 0, Button1, focuswin, {0}},
    {ClkTabPrev, 0, Button1, movestack, {.i = -1}},
    {ClkTabNext, 0, Button1, movestack, {.i = +1}},
    {ClkTabClose, 0, Button1, killclient, {0}},
};
const int buttons_len = sizeof(buttons) / sizeof(buttons[0]);
