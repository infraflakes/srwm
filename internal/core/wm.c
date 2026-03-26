#include "wm.h"

const char broken[] = "broken";

/* function implementations */
int applysizehints(Client* c, int* x, int* y, int* w, int* h, int interact) {
  *w = MAX(1, *w);
  *h = MAX(1, *h);
  if (interact) {
    if (*x > sw) *x = sw - WIDTH(c);
    if (*y > sh) *y = sh - HEIGHT(c);
    if (*x + *w + 2 * c->bw < 0) *x = 0;
    if (*y + *h + 2 * c->bw < 0) *y = 0;
  }
  if (*h < bh) *h = bh;
  if (*w < bh) *w = bh;
  return *x != c->x || *y != c->y || *w != c->w || *h != c->h;
}

void arrange(Monitor* m) {
  if (m)
    showhide(m);
  else
    for (m = mons; m; m = m->next) showhide(m);
  if (m) {
    arrangemon(m);
    restack(m);
  } else
    for (m = mons; m; m = m->next) arrangemon(m);
}

void arrangemon(Monitor* m) {  
  updatebarpos(m);
  updatesystray();

  if (th > 0) {
  /* Position tab bar respecting topbar and toptab */
  if (m->toptab) {
    /* Tab at top of window area */
    m->ty = m->wy;
    m->wy = m->wy + th + m->gap;
    m->wh = m->wh - th - m->gap;
  } else {
    /* Tab at bottom of window area */
    m->wh = m->wh - th - m->gap;
    m->ty = m->wy + m->wh + m->gap;
  }
  XMoveResizeWindow(dpy, m->tabwin, m->wx + m->gap, m->ty,
                    m->ww - 2 * m->gap, th);
  }
}

void attach(Client* c) {
    c->next = NULL;
    if (c->mon->tail) {
        c->mon->tail->next = c;
    } else {
        c->mon->clients = c;
    }
    c->mon->tail = c;
}


void configure(Client* c) {
  XConfigureEvent ce;

  ce.type = ConfigureNotify;
  ce.display = dpy;
  ce.event = c->win;
  ce.window = c->win;
  ce.x = c->x;
  ce.y = c->y;
  ce.width = c->w;
  ce.height = c->h;
  ce.border_width = c->bw;
  ce.above = None;
  ce.override_redirect = False;
  XSendEvent(dpy, c->win, False, StructureNotifyMask, (XEvent*)&ce);
}

void detach(Client* c) {
  Client** tc;
  for (tc = &c->mon->clients; *tc && *tc != c; tc = &(*tc)->next);
  *tc = c->next;
  if (c == c->mon->tail) {
    // Find new tail (previous node or NULL)
    if (c->mon->clients) {
      Client* t = c->mon->clients;
      while (t->next) t = t->next;
      c->mon->tail = t;
    } else {
      c->mon->tail = NULL;
    }
  }
}


void focus(Client* c) {
  if (!c || !ISVISIBLE(c) || !c->ismapped)
    for (c = selmon->clients; c && (!ISVISIBLE(c) || !c->ismapped); c = c->next);
  if (selmon->sel && selmon->sel != c) unfocus(selmon->sel, 0);
  if (c) {
    if (c->mon != selmon) selmon = c->mon;
    if (c->isurgent) seturgent(c, 0);
    grabbuttons(c, 1);
    XSetWindowBorder(dpy, c->win, scheme[SchemeSel][ColBorder].pixel);
    setfocus(c);
  } else {
    XSetInputFocus(dpy, root, RevertToPointerRoot, CurrentTime);
    XDeleteProperty(dpy, root, netatom[NetActiveWindow]);
  }
  selmon->sel = c;
  drawbars();
  drawtabs();
}

void unfocus(Client* c, int setfocus) {
  if (!c) return;
  grabbuttons(c, 0);
  XSetWindowBorder(dpy, c->win, scheme[SchemeNorm][ColBorder].pixel);
  if (setfocus) {
    XSetInputFocus(dpy, root, RevertToPointerRoot, CurrentTime);
    XDeleteProperty(dpy, root, netatom[NetActiveWindow]);
  }
}

void setfocus(Client* c) {
  if (!c->neverfocus) {
    XSetInputFocus(dpy, c->win, RevertToPointerRoot, CurrentTime);
    XChangeProperty(dpy, root, netatom[NetActiveWindow], XA_WINDOW, 32,
                    PropModeReplace, (unsigned char*)&(c->win), 1);
  }
  sendevent(c->win, wmatom[WMTakeFocus], NoEventMask, wmatom[WMTakeFocus],
            CurrentTime, 0, 0, 0);
}

void focusstack(const Arg* arg) {
  if (!selmon->sel || selmon->ntabs < 2) return;

  int idx = -1;
  for (int i = 0; i < selmon->ntabs; i++) {
    if (selmon->tab_order[i] == selmon->sel) { idx = i; break; }
  }
  if (idx < 0) return;

  int target = (idx + arg->i + selmon->ntabs) % selmon->ntabs;
  Client *c = selmon->tab_order[target];
  if (c) {
    focus(c);
    restack(selmon);
  }
}

void focuswin(const Arg* arg) {
  int idx = arg->i;
  if (idx < 0 || idx >= selmon->ntabs) return;
  Client *c = selmon->tab_order[idx];
  if (c) {
    focus(c);
    centerwindowoncanvas(&(Arg){0});
    restack(selmon);
  }
}

void freeicon(Client* c) {
  if (c->icon) {
    XRenderFreePicture(dpy, c->icon);
    c->icon = None;
  }
}

Atom getatomprop(Client* c, Atom prop) {
  int di;
  unsigned long dl;
  unsigned char* p = NULL;
  Atom da, atom = None;
  /* FIXME getatomprop should return the number of items and a pointer to
   * the stored data instead of this workaround */
  Atom req = XA_ATOM;
  if (prop == xatom[XembedInfo]) req = xatom[XembedInfo];

  if (XGetWindowProperty(dpy, c->win, prop, 0L, sizeof atom, False, req, &da,
                         &di, &dl, &dl, &p) == Success &&
      p) {
    atom = *(Atom*)p;
    if (da == xatom[XembedInfo] && dl == 2) atom = ((Atom*)p)[1];
    XFree(p);
  }
  return atom;
}

int getrootptr(int* x, int* y) {
  int di;
  unsigned int dui;
  Window dummy;

  return XQueryPointer(dpy, root, &dummy, &dummy, x, y, &di, &di, &dui);
}

long getstate(Window w) {
  int format;
  long result = -1;
  unsigned char* p = NULL;
  unsigned long n, extra;
  Atom real;

  if (XGetWindowProperty(dpy, w, wmatom[WMState], 0L, 2L, False,
                         wmatom[WMState], &real, &format, &n, &extra,
                         (unsigned char**)&p) != Success)
    return -1;
  if (n != 0) result = *p;
  XFree(p);
  return result;
}

int gettextprop(Window w, Atom atom, char* text, unsigned int size) {
  char** list = NULL;
  int n;
  XTextProperty name;
  static Atom utf8string = None;

  if (!text || size == 0) return 0;
  text[0] = '\0';
  if (!XGetTextProperty(dpy, w, &name, atom) || !name.nitems) return 0;

  if (utf8string == None)
    utf8string = XInternAtom(dpy, "UTF8_STRING", False);

  if (name.encoding == XA_STRING || name.encoding == utf8string) {
    /* XA_STRING (Latin-1) and UTF8_STRING are both byte-safe to copy directly */
    strncpy(text, (char*)name.value, size - 1);
  } else if (XmbTextPropertyToTextList(dpy, &name, &list, &n) >= Success && n > 0 && *list) {
    strncpy(text, *list, size - 1);
    XFreeStringList(list);
  }
  text[size - 1] = '\0';
  XFree(name.value);
  return 1;
}

void killclient(const Arg* arg) {
  if (!selmon->sel) return;
  if (!sendevent(selmon->sel->win, wmatom[WMDelete], NoEventMask,
                 wmatom[WMDelete], CurrentTime, 0, 0, 0)) {
    XGrabServer(dpy);
    XSetErrorHandler(xerrordummy);
    XSetCloseDownMode(dpy, DestroyAll);
    XKillClient(dpy, selmon->sel->win);
    XSync(dpy, False);
    XSetErrorHandler(xerror);
    XUngrabServer(dpy);
  }
}

void manage(Window w, XWindowAttributes* wa) {
  Client *c, *t = NULL;
  Window trans = None;
  XWindowChanges wc;

  c = ecalloc(1, sizeof(Client));
  c->ismapped = 0;
  c->win = w;
  win_ht_insert(c->win, c);
  /* geometry */
  c->x = c->oldx = wa->x;
  c->y = c->oldy = wa->y;
  c->w = c->oldw = wa->width;
  c->h = c->oldh = wa->height;
  c->oldbw = wa->border_width;

  updateicon(c);
  updatetitle(c);
  if (XGetTransientForHint(dpy, w, &trans) && (t = wintoclient(trans))) {
    c->mon = t->mon;
    c->ws = t->ws;
  } else {
    c->mon = selmon;
    c->ws = c->mon->current_ws;
  }

  if (c->x + WIDTH(c) > c->mon->wx + c->mon->ww)
    c->x = c->mon->wx + c->mon->ww - WIDTH(c);
  if (c->y + HEIGHT(c) > c->mon->wy + c->mon->wh)
    c->y = c->mon->wy + c->mon->wh - HEIGHT(c);
  c->x = MAX(c->x, c->mon->wx);
  c->y = MAX(c->y, c->mon->wy);
  c->bw = c->mon->borderpx;

  wc.border_width = c->bw;
  XConfigureWindow(dpy, w, CWBorderWidth, &wc);
  XSetWindowBorder(dpy, w, scheme[SchemeNorm][ColBorder].pixel);
  configure(c); /* propagates border_width, if size doesn't change */
  updatewindowtype(c);
  updatewmhints(c);
  {
    int format;
    unsigned long *data, n, extra;
    Monitor* m;
    Atom atom;
    if (XGetWindowProperty(dpy, c->win, netatom[NetClientInfo], 0L, 2L, False,
                           XA_CARDINAL, &atom, &format, &n, &extra,
                           (unsigned char**)&data) == Success &&
        n == 2) {
      c->ws = (int)*data;
      for (m = mons; m; m = m->next) {
        if (m->num == *(data + 1)) {
          c->mon = m;
          break;
        }
      }
    }
  if (n > 0) XFree(data);
  }
  
  setclientwsprop(c);

  XSelectInput(dpy, w,
               EnterWindowMask | FocusChangeMask | PropertyChangeMask |
                   StructureNotifyMask);
  grabbuttons(c, 0);

  /* Center new window on current viewport so it appears where the user is looking,
     not at the absolute origin which may be far off-screen after panning */
  c->x = c->mon->wx + (c->mon->ww - WIDTH(c)) / 2;
  c->y = c->mon->wy + (c->mon->wh - HEIGHT(c)) / 2;

  /* Scale new window to match current zoom level */
  float zoom = c->mon->canvas_zoom;
  if (zoom != 1.0f && !compositor_running()) {
      int cx = c->mon->wx + c->mon->ww / 2;
      int cy = c->mon->wy + c->mon->wh / 2;
      c->w = MAX(1, (int)(c->w * zoom));
      c->h = MAX(1, (int)(c->h * zoom));
      /* Re-center with new size */
      c->x = cx - (c->w + 2 * c->bw) / 2;
      c->y = cy - (c->h + 2 * c->bw) / 2;
  }
  XRaiseWindow(dpy, c->win);
  attach(c);
  if (ISVISIBLE(c) && c->mon->ntabs < MAXTABS)
    c->mon->tab_order[c->mon->ntabs++] = c;
  XChangeProperty(dpy, root, netatom[NetClientList], XA_WINDOW, 32,
                  PropModeAppend, (unsigned char*)&(c->win), 1);
  XMoveResizeWindow(dpy, c->win, c->x + 2 * sw, c->y, c->w,
                    c->h); /* some windows require this */
  setclientstate(c, NormalState);
  if (c->mon == selmon) unfocus(selmon->sel, 0);
  c->mon->sel = c;
  arrange(c->mon);
  XMapWindow(dpy, c->win);
  c->ismapped = 1;
  focus(NULL);
}

void movestack(const Arg *arg) {
  if (!selmon->sel || selmon->ntabs < 2) return;
  int idx = -1;
  for (int i = 0; i < selmon->ntabs; i++) {
    if (selmon->tab_order[i] == selmon->sel) { idx = i; break; }
  }
  if (idx < 0) return;
  int target = (idx + arg->i + selmon->ntabs) % selmon->ntabs;
  Client *tmp = selmon->tab_order[idx];
  selmon->tab_order[idx] = selmon->tab_order[target];
  selmon->tab_order[target] = tmp;
  drawtabs();	
}

Monitor* recttomon(int x, int y, int w, int h) {
  Monitor *m, *r = selmon;
  int a, area = 0;

  for (m = mons; m; m = m->next)
    if ((a = INTERSECT(x, y, w, h, m)) > area) {
      area = a;
      r = m;
    }
  return r;
}

void resize(Client* c, int x, int y, int w, int h, int interact) {
  if (applysizehints(c, &x, &y, &w, &h, interact)) resizeclient(c, x, y, w, h);
}

void resizeclient(Client* c, int x, int y, int w, int h) {
  XWindowChanges wc;

  c->oldx = c->x;
  c->x = wc.x = x;
  c->oldy = c->y;
  c->y = wc.y = y;
  c->oldw = c->w;
  c->w = wc.width = w;
  c->oldh = c->h;
  c->h = wc.height = h;

  wc.border_width = c->bw;
  XConfigureWindow(dpy, c->win, CWX | CWY | CWWidth | CWHeight | CWBorderWidth,
                   &wc);
  configure(c);
  XSync(dpy, False);
}

void restack(Monitor* m) {
  XEvent ev;

  drawbar(m);
  drawtab(m);
  if (!m->sel) return;
  XRaiseWindow(dpy, m->sel->win);

  /* Always raise bar and tab windows so they stay visible on top */
  XRaiseWindow(dpy, m->barwin);
  if (m->tabwin)
      XRaiseWindow(dpy, m->tabwin);

  XSync(dpy, False);
  while (XCheckMaskEvent(dpy, EnterWindowMask, &ev));
}

void sendmon(Client* c, Monitor* m) {
  if (c->mon == m) return;
  Monitor *old = c->mon;
  unfocus(c, 1);
  /* Remove from source tab_order */
  for (int i = 0; i < old->ntabs; i++) {
    if (old->tab_order[i] == c) {
      memmove(&old->tab_order[i], &old->tab_order[i+1], (old->ntabs - i - 1) * sizeof(Client*));
      old->ntabs--;
      break;
    }
  }
  detach(c);
  c->mon = m;
  c->ws = m->current_ws;
  attach(c);
  /* Add to dest tab_order */
  if (ISVISIBLE(c) && m->ntabs < MAXTABS)
    m->tab_order[m->ntabs++] = c;
  setclientwsprop(c);
  focus(NULL);
  arrange(NULL);
}

void setclientstate(Client* c, long state) {
  long data[] = {state, None};

  XChangeProperty(dpy, c->win, wmatom[WMState], wmatom[WMState], 32,
                  PropModeReplace, (unsigned char*)data, 2);
}

void setclientwsprop(Client* c) {
  long data[] = {(long)c->ws, (long)c->mon->num};
  XChangeProperty(dpy, c->win, netatom[NetClientInfo], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 2);
}

void setfullscreen(Client* c, int fullscreen) {
  if (fullscreen && !c->isfullscreen) {
    XChangeProperty(dpy, c->win, netatom[NetWMState], XA_ATOM, 32,
                    PropModeReplace, (unsigned char*)&netatom[NetWMFullscreen],
                    1);
    c->isfullscreen = 1;
    c->oldbw = c->bw;
    c->bw = 0;
    resizeclient(c, c->mon->mx, c->mon->my, c->mon->mw, c->mon->mh);
    XRaiseWindow(dpy, c->win);
  } 
  else if (!fullscreen && c->isfullscreen) {  
    XDeleteProperty(dpy, c->win, netatom[NetWMState]);
    c->isfullscreen = 0;
    c->bw = c->oldbw;
    c->x = c->oldx;
    c->y = c->oldy;
    c->w = c->oldw;
    c->h = c->oldh;
    resizeclient(c, c->x, c->y, c->w, c->h);
    arrange(c->mon);
  }
}

void seturgent(Client* c, int urg) {
  XWMHints* wmh;

  c->isurgent = urg;
  if (!(wmh = XGetWMHints(dpy, c->win))) return;
  wmh->flags = urg ? (wmh->flags | XUrgencyHint) : (wmh->flags & ~XUrgencyHint);
  XSetWMHints(dpy, c->win, wmh);
  XFree(wmh);
}

void showhide(Monitor *m) {
  XGrabServer(dpy);
  for (Client *c = m->clients; c; c = c->next) {
    if (ISVISIBLE(c)) {
      if (!c->ismapped) {
        window_map(c, 1);
        c->ismapped = 1;
      }
    } else {
      if (c->ismapped) {
        window_unmap(c->win, 1);
        c->ismapped = 0;
      }
    }
  }
  XUngrabServer(dpy);
  XSync(dpy, False);
}

/* WINDOWMAP helpers */  
void window_set_state(Window win, long state) {  
  long data[] = { state, None };  
  XChangeProperty(dpy, win, wmatom[WMState], wmatom[WMState], 32,  
    PropModeReplace, (unsigned char*)data, 2);  
}  
  
void window_map(Client *c, int deiconify) {  
  if (deiconify)  
    window_set_state(c->win, NormalState);
  XMoveResizeWindow(dpy, c->win, c->x, c->y, c->w, c->h);  
  XMapWindow(dpy, c->win);
}  
  
void window_unmap(Window win, int iconify) {  
  static XWindowAttributes ca, ra;  
  XGetWindowAttributes(dpy, root, &ra);  
  XGetWindowAttributes(dpy, win, &ca);  
  /* Prevent UnmapNotify events from reaching our handler */  
  XSelectInput(dpy, root, ra.your_event_mask & ~SubstructureNotifyMask);  
  XSelectInput(dpy, win, ca.your_event_mask & ~StructureNotifyMask);  
  XUnmapWindow(dpy, win);
  if (iconify)  
    window_set_state(win, IconicState);  
  XSelectInput(dpy, root, ra.your_event_mask);  
  XSelectInput(dpy, win, ca.your_event_mask);  
}

void unmanage(Client* c, int destroyed) {
  Monitor* m = c->mon;
  XWindowChanges wc;

  /* Remove from tab_order */
  for (int i = 0; i < m->ntabs; i++) {
    if (m->tab_order[i] == c) {
      memmove(&m->tab_order[i], &m->tab_order[i+1], (m->ntabs - i - 1) * sizeof(Client*));
      m->ntabs--;
      break;
    }
  }

  /* If this was the selected client, find next visible one */
  if (c == m->sel) {
    Client *t;
    for (t = m->clients; t && (!ISVISIBLE(t) || !t->ismapped || t == c); t = t->next);
    m->sel = t;
  }

  detach(c);
  freeicon(c);

  if (!destroyed) {
    wc.border_width = c->oldbw;
    XGrabServer(dpy); /* avoid race conditions */
    XSetErrorHandler(xerrordummy);
    XSelectInput(dpy, c->win, NoEventMask);
    XConfigureWindow(dpy, c->win, CWBorderWidth, &wc); /* restore border */
    XUngrabButton(dpy, AnyButton, AnyModifier, c->win);
    setclientstate(c, WithdrawnState);
    XSync(dpy, False);
    XSetErrorHandler(xerror);
    XUngrabServer(dpy);
  }
  win_ht_remove(c->win);
  free(c);
  focus(NULL);
  updateclientlist();
  arrange(m);
}

void updateclientlist(void) {
  Client* c;
  Monitor* m;

  XDeleteProperty(dpy, root, netatom[NetClientList]);
  for (m = mons; m; m = m->next)
    for (c = m->clients; c; c = c->next)
      XChangeProperty(dpy, root, netatom[NetClientList], XA_WINDOW, 32,
                      PropModeAppend, (unsigned char*)&(c->win), 1);
}

void updatetitle(Client* c) {
  if (!gettextprop(c->win, netatom[NetWMName], c->name, sizeof c->name))
    gettextprop(c->win, XA_WM_NAME, c->name, sizeof c->name);
  if (c->name[0] == '\0') /* hack to mark broken clients */
    strcpy(c->name, broken);
}

void updatewindowtype(Client* c) {
  Atom state = getatomprop(c, netatom[NetWMState]);
  if (state == netatom[NetWMFullscreen]) setfullscreen(c, 1);
}

void rebuild_tab_order(Monitor *m) {
  m->ntabs = 0;
  for (Client *c = m->clients; c; c = c->next) {
    if (ISVISIBLE(c) && m->ntabs < MAXTABS)
      m->tab_order[m->ntabs++] = c;
  }
}

void updatewmhints(Client* c) {
  XWMHints* wmh;

  if ((wmh = XGetWMHints(dpy, c->win))) {
    if (c == selmon->sel && wmh->flags & XUrgencyHint) {
      wmh->flags &= ~XUrgencyHint;
      XSetWMHints(dpy, c->win, wmh);
    } else
      c->isurgent = (wmh->flags & XUrgencyHint) ? 1 : 0;
    if (wmh->flags & InputHint)
      c->neverfocus = !wmh->input;
    else
      c->neverfocus = 0;
    XFree(wmh);
  }
}

void updateicon(Client* c) {
  freeicon(c);
  c->icon = geticonprop(c->win, &c->icw, &c->ich);
}

/* Window -> Client hash table for O(1) lookup */
#define WIN_HT_SIZE 256  /* must be power of 2 */
static struct { Window win; Client* c; } win_ht[WIN_HT_SIZE];
unsigned int win_hash(Window w) {
    return (unsigned int)(w * 2654435761u) & (WIN_HT_SIZE - 1);
}

void win_ht_insert(Window w, Client* c) {
    unsigned int h = win_hash(w);
    while (win_ht[h].win && win_ht[h].win != w)
        h = (h + 1) & (WIN_HT_SIZE - 1);
    win_ht[h].win = w;
    win_ht[h].c = c;
}

void win_ht_remove(Window w) {
    unsigned int h = win_hash(w);
    while (win_ht[h].win) {
        if (win_ht[h].win == w) {
            win_ht[h].win = 0;
            win_ht[h].c = NULL;
            /* rehash subsequent entries in cluster */
            unsigned int j = (h + 1) & (WIN_HT_SIZE - 1);
            while (win_ht[j].win) {
                Window tw = win_ht[j].win;
                Client* tc = win_ht[j].c;
                win_ht[j].win = 0;
                win_ht[j].c = NULL;
                win_ht_insert(tw, tc);
                j = (j + 1) & (WIN_HT_SIZE - 1);
            }
            return;
        }
        h = (h + 1) & (WIN_HT_SIZE - 1);
    }
}

Client* wintoclient(Window w) {
    unsigned int h = win_hash(w);
    while (win_ht[h].win) {
        if (win_ht[h].win == w) return win_ht[h].c;
        h = (h + 1) & (WIN_HT_SIZE - 1);
    }
    return NULL;
}

Monitor* wintomon(Window w) {
  int x, y;
  Client* c;
  Monitor* m;

  if (w == root && getrootptr(&x, &y)) return recttomon(x, y, 1, 1);
  for (m = mons; m; m = m->next)
    if (w == m->barwin || w == m->tabwin) return m;
  if ((c = wintoclient(w))) return c->mon;
  return selmon;
}

void grabbuttons(Client* c, int focused) {
  updatenumlockmask();
  {
    unsigned int i, j;
    unsigned int modifiers[] = {0, LockMask, numlockmask,
                                numlockmask | LockMask};
    XUngrabButton(dpy, AnyButton, AnyModifier, c->win);
    if (!focused)
      XGrabButton(dpy, AnyButton, AnyModifier, c->win, False, BUTTONMASK,
                  GrabModeSync, GrabModeSync, None, None);
    for (i = 0; i < buttons_len; i++)
      if (buttons[i].click == ClkClientWin)
        for (j = 0; j < LENGTH(modifiers); j++)
          XGrabButton(dpy, buttons[i].button, buttons[i].mask | modifiers[j],
                      c->win, False, BUTTONMASK, GrabModeAsync, GrabModeSync,
                      None, None);
    for (i = 0; i < (unsigned int)dbuttons_len; i++)
      if (dbuttons[i].click == ClkClientWin)
        for (j = 0; j < LENGTH(modifiers); j++)
          XGrabButton(dpy, dbuttons[i].button, dbuttons[i].mod | modifiers[j],
                      c->win, False, BUTTONMASK, GrabModeAsync, GrabModeSync,
                      None, None);
  }
}


