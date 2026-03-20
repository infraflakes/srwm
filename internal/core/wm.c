#include "wm.h"

const char broken[] = "broken";

/* function implementations */
void applyrules(Client* c) {
  const char* class, *instance;
  unsigned int i;
  const Rule* r;
  Monitor* m;
  XClassHint ch = {NULL, NULL};

  /* rule matching */
  c->iscentered = 0;
  c->isfloating = 0;
  c->tags = 0;
  XGetClassHint(dpy, c->win, &ch);
  class = ch.res_class ? ch.res_class : broken;
  instance = ch.res_name ? ch.res_name : broken;

  for (i = 0; i < LENGTH(rules); i++) {
    r = &rules[i];
    if ((!r->title || strstr(c->name, r->title)) &&
        (!r->class || strstr(class, r->class)) &&
        (!r->instance || strstr(instance, r->instance))) {
      c->iscentered = r->iscentered;
      c->isfloating = r->isfloating;
      c->tags |= r->tags;
      for (m = mons; m && m->num != r->monitor; m = m->next);
      if (m) c->mon = m;
    }
  }
  if (ch.res_class) XFree(ch.res_class);
  if (ch.res_name) XFree(ch.res_name);
  c->tags =
      c->tags & TAGMASK ? c->tags & TAGMASK : c->mon->tagset[c->mon->seltags];
}

int applysizehints(Client* c, int* x, int* y, int* w, int* h, int interact) {
  int baseismin;
  Monitor* m = c->mon;

  /* set minimum possible */
  *w = MAX(1, *w);
  *h = MAX(1, *h);
  if (interact) {
    if (*x > sw) *x = sw - WIDTH(c);
    if (*y > sh) *y = sh - HEIGHT(c);
    if (*x + *w + 2 * c->bw < 0) *x = 0;
    if (*y + *h + 2 * c->bw < 0) *y = 0;
  } else {
    if (*x >= m->wx + m->ww) *x = m->wx + m->ww - WIDTH(c);
    if (*y >= m->wy + m->wh) *y = m->wy + m->wh - HEIGHT(c);
    if (*x + *w + 2 * c->bw <= m->wx) *x = m->wx;
    if (*y + *h + 2 * c->bw <= m->wy) *y = m->wy;
  }
  if (*h < bh) *h = bh;
  if (*w < bh) *w = bh;
  if (c->isfloating) {
    if (!c->hintsvalid) updatesizehints(c);
    /* see last two sentences in ICCCM 4.1.2.3 */
    baseismin = c->basew == c->minw && c->baseh == c->minh;
    if (!baseismin) { /* temporarily remove base dimensions */
      *w -= c->basew;
      *h -= c->baseh;
    }
    /* adjust for aspect limits */
    if (c->mina > 0 && c->maxa > 0) {
      if (c->maxa < (float)*w / *h)
        *w = *h * c->maxa + 0.5;
      else if (c->mina < (float)*h / *w)
        *h = *w * c->mina + 0.5;
    }
    if (baseismin) { /* increment calculation requires this */
      *w -= c->basew;
      *h -= c->baseh;
    }
    /* adjust for increment value */
    if (c->incw) *w -= *w % c->incw;
    if (c->inch) *h -= *h % c->inch;
    /* restore base dimensions */
    *w = MAX(*w + c->basew, c->minw);
    *h = MAX(*h + c->baseh, c->minh);
    if (c->maxw) *w = MIN(*w, c->maxw);
    if (c->maxh) *h = MIN(*h, c->maxh);
  }
  return *x != c->x || *y != c->y || *w != c->w || *h != c->h;
}

void arrange(Monitor* m) {
  if (m)
    showhide(m->stack);
  else
    for (m = mons; m; m = m->next) showhide(m->stack);
  if (m) {
    arrangemon(m);
    restack(m);
  } else
    for (m = mons; m; m = m->next) arrangemon(m);
}

void arrangemon(Monitor* m) {  
  Client* c;
  int newx, newy, neww, newh;

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

  /* Skip tiling in canvas mode — all windows are floating */  
  if (m->canvas_mode) return;

  /* Arrange all tiled windows in fullscreen style */
  for (c = nexttiled(m->clients); c; c = nexttiled(c->next)) {
    newx = m->wx + m->gap - c->bw;
    newy = m->wy + m->gap - c->bw;
    neww = m->ww - 2 * (m->gap + c->bw);
    newh = m->wh - 2 * (m->gap + c->bw);
    applysizehints(c, &newx, &newy, &neww, &newh, 0);
    if (neww < m->ww) newx = m->wx + (m->ww - (neww + 2 * c->bw)) / 2;
    if (newh < m->wh) newy = m->wy + (m->wh - (newh + 2 * c->bw)) / 2;
    resize(c, newx, newy, neww, newh, 0);
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
    c->mon->occ |= c->tags;
}

void attachstack(Client* c) {
  c->snext = c->mon->stack;
  c->mon->stack = c;
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
  // Recompute occ (can't just clear bits — other clients may share tags)
  c->mon->occ = 0;
  for (Client* t = c->mon->clients; t; t = t->next)
  c->mon->occ |= t->tags;
}

void detachstack(Client* c) {
  Client **tc, *t;

  for (tc = &c->mon->stack; *tc && *tc != c; tc = &(*tc)->snext);
  *tc = c->snext;

  if (c == c->mon->sel) {
    for (t = c->mon->stack; t && !ISVISIBLE(t); t = t->snext);
    c->mon->sel = t;
  }
}

void focus(Client* c) {
  if (!c || (!ISVISIBLE(c) || HIDDEN(c)))
    for (c = selmon->stack; c && (!ISVISIBLE(c) || HIDDEN(c)); c = c->snext);
  if (selmon->sel && selmon->sel != c) unfocus(selmon->sel, 0);
  if (c) {
    if (c->mon != selmon) selmon = c->mon;
    if (c->isurgent) seturgent(c, 0);
    detachstack(c);
    attachstack(c);
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
  Client *c = NULL, *i;

  if (!selmon->sel) return;
  if (arg->i > 0) {
    for (c = selmon->sel->next; c && (!ISVISIBLE(c) || HIDDEN(c)); c = c->next);
    if (!c)
      for (c = selmon->clients; c && (!ISVISIBLE(c) || HIDDEN(c)); c = c->next);
  } else {
    for (i = selmon->clients; i != selmon->sel; i = i->next)
      if (ISVISIBLE(i) && !HIDDEN(i)) c = i;
    if (!c)
      for (; i; i = i->next)
        if (ISVISIBLE(i) && !HIDDEN(i)) c = i;
  }
  if (c) {
    focus(c);
    restack(selmon);
  }
}

void focuswin(const Arg* arg) {
  int iwin = arg->i;
  Client* c = NULL;
  for (c = selmon->clients; c && (iwin || !ISVISIBLE(c)); c = c->next) {
    if (ISVISIBLE(c)) --iwin;
  };
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
  c->ishidden = 0; //new windows are not hidden
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
    c->tags = t->tags;
  } else {
    c->mon = selmon;
    applyrules(c);
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
  updatesizehints(c);
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
      c->tags = *data;
      for (m = mons; m; m = m->next) {
        if (m->num == *(data + 1)) {
          c->mon = m;
          break;
        }
      }
    }
    if (n > 0) XFree(data);
  }
  setclienttagprop(c);

  if (c->iscentered) {
    c->x = c->mon->mx + (c->mon->mw - WIDTH(c)) / 2;
    c->y = c->mon->my + (c->mon->mh - HEIGHT(c)) / 2;
  }
  XSelectInput(dpy, w,
               EnterWindowMask | FocusChangeMask | PropertyChangeMask |
                   StructureNotifyMask);
  grabbuttons(c, 0);
  if (!c->isfloating) c->isfloating = c->oldstate = trans != None || c->isfixed;
  /* In canvas mode, all new windows are floating */  
  if (c->mon->canvas_mode && !c->isfloating) {  
     c->isfloating = 1;  
  }
  if (c->mon->canvas_mode) {  
    /* Center new window on current viewport so it appears where the user is looking,  
       not at the absolute origin which may be far off-screen after panning */  
    c->x = c->mon->wx + (c->mon->ww - WIDTH(c)) / 2;  
    c->y = c->mon->wy + (c->mon->wh - HEIGHT(c)) / 2;  

    /* Scale new window to match current zoom level */
    int tagidx = getcurrenttag(c->mon);
    float zoom = c->mon->canvas[tagidx].zoom;
    if (zoom != 1.0f) {
        int cx = c->mon->wx + c->mon->ww / 2;
        int cy = c->mon->wy + c->mon->wh / 2;
        c->w = MAX(1, (int)(c->w * zoom));
        c->h = MAX(1, (int)(c->h * zoom));
        /* Re-center with new size */
        c->x = cx - (c->w + 2 * c->bw) / 2;
        c->y = cy - (c->h + 2 * c->bw) / 2;
    }
  }
  if (c->isfloating) XRaiseWindow(dpy, c->win);
  attach(c);
  attachstack(c);
  XChangeProperty(dpy, root, netatom[NetClientList], XA_WINDOW, 32,
                  PropModeAppend, (unsigned char*)&(c->win), 1);
  XMoveResizeWindow(dpy, c->win, c->x + 2 * sw, c->y, c->w,
                    c->h); /* some windows require this */
  if (!HIDDEN(c)) setclientstate(c, NormalState);
  if (c->mon == selmon) unfocus(selmon->sel, 0);
  c->mon->sel = c;
  arrange(c->mon);
  if (!HIDDEN(c)) XMapWindow(dpy, c->win);
  if (!HIDDEN(c)) c->ismapped = 1;
  focus(NULL);
}

void movestack(const Arg *arg) {
	Client *c = NULL, *p = NULL, *pc = NULL, *i;
	int skip_float = !selmon->canvas_mode;

	if(arg->i > 0) {
		/* find the client after selmon->sel */
		for(c = selmon->sel->next; c && (!ISVISIBLE(c) || (skip_float && c->isfloating)); c = c->next);
		if(!c)
			for(c = selmon->clients; c && (!ISVISIBLE(c) || (skip_float && c->isfloating)); c = c->next);

	}
	else {
		/* find the client before selmon->sel */
		for(i = selmon->clients; i != selmon->sel; i = i->next)
			if(ISVISIBLE(i) && !(skip_float && i->isfloating))
				c = i;
		if(!c)
			for(; i; i = i->next)
				if(ISVISIBLE(i) && !(skip_float && i->isfloating))
					c = i;
	}
	/* find the client before selmon->sel and c */
	for(i = selmon->clients; i && (!p || !pc); i = i->next) {
		if(i->next == selmon->sel)
			p = i;
		if(i->next == c)
			pc = i;
	}

	/* swap c and selmon->sel selmon->clients in the selmon->clients list */
	if(c && c != selmon->sel) {
		Client *temp = selmon->sel->next==c?selmon->sel:selmon->sel->next;
		selmon->sel->next = c->next==selmon->sel?c:c->next;
		c->next = temp;

		if(p && p != c)
			p->next = c;
		if(pc && pc != selmon->sel)
			pc->next = selmon->sel;

		if(selmon->sel == selmon->clients)
			selmon->clients = c;
		else if(c == selmon->clients)
			selmon->clients = selmon->sel;

		arrange(selmon);
	}
}

Client* nexttiled(Client* c) {
  for (; c && (c->isfloating || (!ISVISIBLE(c) || HIDDEN(c))); c = c->next);
  return c;
}

Client* recttoclient(int x, int y, int w, int h) {
  Client *c, *r = NULL;
  int a, area = 0;

  for (c = nexttiled(selmon->clients); c; c = nexttiled(c->next)) {
    if ((a = INTERSECTC(x, y, w, h, c)) > area) {
      area = a;
      r = c;
    }
  }
  return r;
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

  if (c->beingmoved) return;

  wc.border_width = c->bw;
  XConfigureWindow(dpy, c->win, CWX | CWY | CWWidth | CWHeight | CWBorderWidth,
                   &wc);
  configure(c);
  XSync(dpy, False);
}

void restack(Monitor* m) {
  Client* c;
  XEvent ev;
  XWindowChanges wc;

  drawbar(m);
  drawtab(m);
  if (!m->sel) return;
  if (m->sel->isfloating) XRaiseWindow(dpy, m->sel->win);
  wc.stack_mode = Below;
  wc.sibling = m->barwin;
  for (c = m->stack; c; c = c->snext)
    if (!c->isfloating && ISVISIBLE(c)) {
      XConfigureWindow(dpy, c->win, CWSibling | CWStackMode, &wc);
      wc.sibling = c->win;
    }
  /* In canvas mode all windows are floating and can overlap the bar.  
   Raise bar and tab windows so they stay visible on top. */  
  if (m->canvas_mode) {  
      XRaiseWindow(dpy, m->barwin);  
      if (m->tabwin)  
          XRaiseWindow(dpy, m->tabwin);  
  }
  XSync(dpy, False);
  while (XCheckMaskEvent(dpy, EnterWindowMask, &ev));
}

void sendmon(Client* c, Monitor* m) {
  if (c->mon == m) return;
  unfocus(c, 1);
  detach(c);
  detachstack(c);
  c->mon = m;
  c->tags = m->tagset[m->seltags]; /* assign tags of target monitor */
  attach(c);
  attachstack(c);
  setclienttagprop(c);
  focus(NULL);
  arrange(NULL);
}

void setclientstate(Client* c, long state) {
  long data[] = {state, None};

  XChangeProperty(dpy, c->win, wmatom[WMState], wmatom[WMState], 32,
                  PropModeReplace, (unsigned char*)data, 2);

  c->ishidden = (state == IconicState);
}

void setclienttagprop(Client* c) {
  long data[] = {(long)c->tags, (long)c->mon->num};
  XChangeProperty(dpy, c->win, netatom[NetClientInfo], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 2);
}

void setfullscreen(Client* c, int fullscreen) {
  if (fullscreen && !c->isfullscreen) {
    XChangeProperty(dpy, c->win, netatom[NetWMState], XA_ATOM, 32,
                    PropModeReplace, (unsigned char*)&netatom[NetWMFullscreen],
                    1);
    c->isfullscreen = 1;
    c->oldstate = c->isfloating;
    c->oldbw = c->bw;
    c->bw = 0;
    c->isfloating = 1;
    resizeclient(c, c->mon->mx, c->mon->my, c->mon->mw, c->mon->mh);
    XRaiseWindow(dpy, c->win);
  } 
  else if (!fullscreen && c->isfullscreen) {  
    XDeleteProperty(dpy, c->win, netatom[NetWMState]);
    c->isfullscreen = 0;
    c->isfloating = c->oldstate;
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
  if (urg) c->mon->urg |= c->tags;
  else {
    c->mon->urg = 0;
    for (Client* t = c->mon->clients; t; t = t->next)
    if (t->isurgent) c->mon->urg |= t->tags;
  }
}

void showhide(Client* c) {  
  if (!c) return;  
  
  /* Grab server on first call to batch all map/unmap operations */  
  static int grabbed = 0;  
  if (!grabbed) {  
    XGrabServer(dpy);  
    grabbed = 1;  
  }  
  
  if (ISVISIBLE(c)) {  
    /* show clients top down */  
    if (!c->ismapped) {  
      window_map(c, 1);  
      c->ismapped = 1;  
    }  
    showhide(c->snext);  
  } else {  
    /* hide clients bottom up */  
    showhide(c->snext);  
    if (c->ismapped) {  
      window_unmap(c->win, 1);  
      c->ismapped = 0;  
    }  
  }  
  
  /* Ungrab server after processing the last client in the stack */  
  if (!c->snext && grabbed) {  
    XUngrabServer(dpy);  
    XSync(dpy, False);  
    grabbed = 0;  
  }  
}

/* WINDOWMAP helpers */  
void window_set_state(Window win, long state) {  
  long data[] = { state, None };  
  XChangeProperty(dpy, win, wmatom[WMState], wmatom[WMState], 32,  
    PropModeReplace, (unsigned char*)data, 2);  
}  
  
void window_map(Client *c, int deiconify) {  
  Window win = c->win;  
  if (deiconify)  
    window_set_state(win, NormalState);  
  XMoveResizeWindow(dpy, c->win, c->x, c->y, c->w, c->h);  
  XSetInputFocus(dpy, win, RevertToPointerRoot, CurrentTime);  
  XMapWindow(dpy, win);  
  focus(NULL);  
}  
  
void window_unmap(Window win, int iconify) {  
  static XWindowAttributes ca, ra;  
  XGetWindowAttributes(dpy, root, &ra);  
  XGetWindowAttributes(dpy, win, &ca);  
  /* Prevent UnmapNotify events from reaching our handler */  
  XSelectInput(dpy, root, ra.your_event_mask & ~SubstructureNotifyMask);  
  XSelectInput(dpy, win, ca.your_event_mask & ~StructureNotifyMask);  
  XUnmapWindow(dpy, win);  
  focus(NULL);  
  if (iconify)  
    window_set_state(win, IconicState);  
  XSelectInput(dpy, root, ra.your_event_mask);  
  XSelectInput(dpy, win, ca.your_event_mask);  
}

void unmanage(Client* c, int destroyed) {
  Monitor* m = c->mon;
  XWindowChanges wc;

  detach(c);
  detachstack(c);
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

void updatesizehints(Client* c) {
  long msize;
  XSizeHints size;

  if (!XGetWMNormalHints(dpy, c->win, &size, &msize))
    /* size is uninitialized, ensure that size.flags aren't used */
    size.flags = PSize;
  if (size.flags & PBaseSize) {
    c->basew = size.base_width;
    c->baseh = size.base_height;
  } else if (size.flags & PMinSize) {
    c->basew = size.min_width;
    c->baseh = size.min_height;
  } else
    c->basew = c->baseh = 0;
  if (size.flags & PResizeInc) {
    c->incw = size.width_inc;
    c->inch = size.height_inc;
  } else
    c->incw = c->inch = 0;
  if (size.flags & PMaxSize) {
    c->maxw = size.max_width;
    c->maxh = size.max_height;
  } else
    c->maxw = c->maxh = 0;
  if (size.flags & PMinSize) {
    c->minw = size.min_width;
    c->minh = size.min_height;
  } else if (size.flags & PBaseSize) {
    c->minw = size.base_width;
    c->minh = size.base_height;
  } else
    c->minw = c->minh = 0;
  if (size.flags & PAspect) {
    c->mina = (float)size.min_aspect.y / size.min_aspect.x;
    c->maxa = (float)size.max_aspect.x / size.max_aspect.y;
  } else
    c->maxa = c->mina = 0.0;
  c->isfixed = (c->maxw && c->maxh && c->maxw == c->minw && c->maxh == c->minh);
  c->hintsvalid = 1;
}

void updatetitle(Client* c) {
  if (!gettextprop(c->win, netatom[NetWMName], c->name, sizeof c->name))
    gettextprop(c->win, XA_WM_NAME, c->name, sizeof c->name);
  if (c->name[0] == '\0') /* hack to mark broken clients */
    strcpy(c->name, broken);
}

void updatewindowtype(Client* c) {
  Atom state = getatomprop(c, netatom[NetWMState]);
  Atom wtype = getatomprop(c, netatom[NetWMWindowType]);

  if (state == netatom[NetWMFullscreen]) setfullscreen(c, 1);
  if (wtype == netatom[NetWMWindowTypeDialog]) {
    c->iscentered = 1;
    c->isfloating = 1;
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
    for (i = 0; i < LENGTH(buttons); i++)
      if (buttons[i].click == ClkClientWin)
        for (j = 0; j < LENGTH(modifiers); j++)
          XGrabButton(dpy, buttons[i].button, buttons[i].mask | modifiers[j],
                      c->win, False, BUTTONMASK, GrabModeAsync, GrabModeSync,
                      None, None);
  }
}


