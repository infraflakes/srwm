#include "wm.h"

Systray* systray = NULL;

void updatesystray(void) {
  XSetWindowAttributes wa;
  XWindowChanges wc;
  Client* i;
  Monitor* m = systraytomon(NULL);
  unsigned int x = m->mx + m->mw - m->gap;
  unsigned int w = 1;

  if (!systray_enable) return;
  if (!systray) {
    /* init systray */
    if (!(systray = (Systray*)calloc(1, sizeof(Systray))))
      die("fatal: could not malloc() %u bytes\n", sizeof(Systray));
    systray->win = XCreateSimpleWindow(dpy, root, x, m->by, w, bh, 0, 0,
                                       scheme[SchemeSel][ColBg].pixel);
    wa.event_mask = ButtonPressMask | ExposureMask;
    wa.override_redirect = True;
    wa.background_pixel = scheme[SchemeNorm][ColBg].pixel;
    XSelectInput(dpy, systray->win, SubstructureNotifyMask);
    XChangeProperty(dpy, systray->win, netatom[NetSystemTrayOrientation],
                    XA_CARDINAL, 32, PropModeReplace,
                    (unsigned char*)&netatom[NetSystemTrayOrientationHorz], 1);
    XChangeWindowAttributes(
        dpy, systray->win, CWEventMask | CWOverrideRedirect | CWBackPixel, &wa);
    XMapRaised(dpy, systray->win);
    XChangeProperty(dpy, systray->win, netatom[NetWMWindowType], XA_ATOM, 32,  
                PropModeReplace, (unsigned char*)&netatom[NetWMWindowTypeDock], 1);
    XClassHint systray_ch = {"srwm", "srwm"};
    XSetClassHint(dpy, systray->win, &systray_ch);
    XSetSelectionOwner(dpy, netatom[NetSystemTray], systray->win, CurrentTime);
    if (XGetSelectionOwner(dpy, netatom[NetSystemTray]) == systray->win) {
      sendevent(root, xatom[Manager], StructureNotifyMask, CurrentTime,
                netatom[NetSystemTray], systray->win, 0, 0);
      XSync(dpy, False);
    } else {
      fprintf(stderr, "srwm: unable to obtain system tray.\n");
      free(systray);
      systray = NULL;
      return;
    }
  }
  for (w = 0, i = systray->icons; i; i = i->next) {
    /* make sure the background color stays the same */
    wa.background_pixel = scheme[SchemeNorm][ColBg].pixel;
    XChangeWindowAttributes(dpy, i->win, CWBackPixel, &wa);
    w += systrayspacing;
    i->x = w;
    XMoveResizeWindow(dpy, i->win, i->x, bar_vertical_padding / 2, i->w, i->h);
    XMapRaised(dpy, i->win);
    w += i->w;
    if (i->mon != m) i->mon = m;
  }
  w = w ? w + systrayspacing : 1;
  x -= w;
  XMoveResizeWindow(dpy, systray->win, x, m->by, w, bh);
  wc.x = x;
  wc.y = m->by;
  wc.width = w;
  wc.height = bh;
  wc.stack_mode = Above;
  wc.sibling = m->barwin;
  XConfigureWindow(dpy, systray->win,
                   CWX | CWY | CWWidth | CWHeight | CWSibling | CWStackMode,
                   &wc);
  XMapWindow(dpy, systray->win);
  XMapSubwindows(dpy, systray->win);
  /* redraw background */
  XSetForeground(dpy, drw->gc, scheme[SchemeNorm][ColBg].pixel);
  XFillRectangle(dpy, systray->win, drw->gc, 0, 0, w, bh);
  XSync(dpy, False);
}

void updatesystrayiconstate(Client* i, XPropertyEvent* ev) {
  long flags;
  int code = 0;

  if (!systray_enable || !i || ev->atom != xatom[XembedInfo] ||
      !(flags = getatomprop(i, xatom[XembedInfo])))
    return;

  if (flags & XEMBED_MAPPED && !i->tags) {
    i->tags = 1;
    code = XEMBED_WINDOW_ACTIVATE;
    XMapRaised(dpy, i->win);
    setclientstate(i, NormalState);
  } else if (!(flags & XEMBED_MAPPED) && i->tags) {
    i->tags = 0;
    code = XEMBED_WINDOW_DEACTIVATE;
    XUnmapWindow(dpy, i->win);
    setclientstate(i, WithdrawnState);
  } else
    return;
  sendevent(i->win, xatom[Xembed], StructureNotifyMask, CurrentTime, code, 0,
            systray->win, XEMBED_EMBEDDED_VERSION);
}

void updatesystrayicongeom(Client* i, int w, int h) {
  int rh = bh - bar_vertical_padding;
  if (i) {
    i->h = rh;
    if (w == h)
      i->w = rh;
    else if (h == rh)
      i->w = w;
    else
      i->w = (int)((float)rh * ((float)w / (float)h));
    if (i->w < 1) i->w = 1;
    if (i->h < 1) i->h = 1;
  }
}

unsigned int getsystraywidth(void) {
  unsigned int w = 0;
  Client* i;
  if (systray_enable)
    for (i = systray->icons; i; w += i->w + systrayspacing, i = i->next);
  return w ? w + systrayspacing : 1;
}

Monitor* systraytomon(Monitor* m) {
  Monitor* t;
  int i, n;
  if (!systraypinning) {
    if (!m) return selmon;
    return m == selmon ? m : NULL;
  }
  for (n = 1, t = mons; t && t->next; n++, t = t->next);
  i = systraypinning - 1;
  if (i >= n) i = 0;
  for (t = mons; t && i > 0; i--, t = t->next);
  return t ? t : mons;
}

void removesystrayicon(Client* i) {
  Client** ii;

  if (!systray_enable || !i) return;
  for (ii = &systray->icons; *ii && *ii != i; ii = &(*ii)->next);
  if (ii) *ii = i->next;
  free(i);
}

Client* wintosystrayicon(Window w) {
  Client* i = NULL;

  if (!systray_enable || !w) return i;
  for (i = systray->icons; i && i->win != w; i = i->next);
  return i;
}


