#include "wm.h"

void (*handler[LASTEvent])(XEvent*) = {  
    [ButtonPress] = buttonpress,  
    [ClientMessage] = clientmessage,  
    [ConfigureRequest] = configurerequest,  
    [ConfigureNotify] = configurenotify,  
    [DestroyNotify] = destroynotify,  
    [EnterNotify] = enternotify,  
    [Expose] = expose,  
    [FocusIn] = focusin,  
    [KeyPress] = keypress,  
    [MappingNotify] = mappingnotify,  
    [MapRequest] = maprequest,  
    [MotionNotify] = motionnotify,  
    [PropertyNotify] = propertynotify,  
    [ResizeRequest] = resizerequest,  
    [UnmapNotify] = unmapnotify  
};
char stext[1024];

void buttonpress(XEvent* e) {
  unsigned int i, x, click;
  int loop;
  Arg arg = {0};
  Client* c;
  Monitor* m;
  XButtonPressedEvent* ev = &e->xbutton;

  click = ClkRootWin;
  /* focus monitor if necessary */
  if ((m = wintomon(ev->window)) && m != selmon) {
    unfocus(selmon->sel, 1);
    selmon = m;
    focus(NULL);
  }
  if (ev->window == selmon->barwin) {
    i = x = 0;
    do x += TEXTW(tags[i]);
    while (ev->x >= x && ++i < TAGSLENGTH);
    if (i < TAGSLENGTH) {
      click = ClkTagBar;
      arg.ui = 1 << i;
      goto execute_handler;
    }

    if (ev->x > selmon->ww - (int)TEXTW(stext))
      click = ClkStatusText;
    else
      click = ClkWinTitle;
  }

  if (ev->window == selmon->tabwin) {
    i = 0;
    x = 0;
    for (c = selmon->clients; c; c = c->next) {
      if (!ISVISIBLE(c)) continue;
      x += selmon->tab_widths[i];
      if (ev->x > x)
        ++i;
      else
        break;
      if (i >= m->ntabs) break;
    }
    if (c && ev->x <= x) {
      click = ClkTabBar;
      arg.ui = i;
    } else {
      x = selmon->ww - 2 * m->gap;
      for (loop = 2; loop >= 0; loop--) {
        x -= selmon->tab_btn_w[loop];
        if (ev->x > x) break;
      }
      if (ev->x >= x) click = ClkTabPrev + loop;
    }
  } else if ((c = wintoclient(ev->window))) {
    focus(c);
    restack(selmon);
    XAllowEvents(dpy, ReplayPointer, CurrentTime);
    click = ClkClientWin;
  }

execute_handler:

  for (i = 0; i < LENGTH(buttons); i++)
    if (click == buttons[i].click && buttons[i].func &&
        buttons[i].button == ev->button &&
        CLEANMASK(buttons[i].mask) == CLEANMASK(ev->state))
      buttons[i].func(
          ((click == ClkTagBar || click == ClkTabBar) && buttons[i].arg.i == 0)
              ? &arg
              : &buttons[i].arg);
}

void clientmessage(XEvent* e) {
  XWindowAttributes wa;
  XSetWindowAttributes swa;
  XClientMessageEvent* cme = &e->xclient;
  Client* c = wintoclient(cme->window);

  if (systray_enable && cme->window == systray->win &&
      cme->message_type == netatom[NetSystemTrayOP]) {
    /* add systray icons */
    if (cme->data.l[1] == SYSTEM_TRAY_REQUEST_DOCK) {
      if (!(c = (Client*)calloc(1, sizeof(Client))))
        die("fatal: could not malloc() %u bytes\n", sizeof(Client));
      if (!(c->win = cme->data.l[2])) {
        free(c);
        return;
      }
      c->mon = selmon;
      c->next = systray->icons;
      systray->icons = c;
      if (!XGetWindowAttributes(dpy, c->win, &wa)) {
        /* use sane defaults */
        wa.width = bh;
        wa.height = bh;
        wa.border_width = 0;
      }
      c->x = c->oldx = c->y = c->oldy = 0;
      c->w = c->oldw = wa.width;
      c->h = c->oldh = wa.height;
      c->oldbw = wa.border_width;
      c->bw = 0;
      c->isfloating = True;
      /* reuse tags field as mapped status */
      c->tags = 1;
      updatesizehints(c);
      updatesystrayicongeom(c, wa.width, wa.height);
      XAddToSaveSet(dpy, c->win);
      XSelectInput(
          dpy, c->win,
          StructureNotifyMask | PropertyChangeMask | ResizeRedirectMask);
      XClassHint ch = {"srwmsystray", "srwmsystray"};
      XSetClassHint(dpy, c->win, &ch);
      XReparentWindow(dpy, c->win, systray->win, 0, 0);
      XResizeWindow(dpy, c->win, c->w, c->h);
      /* use parents background color */
      swa.background_pixel = scheme[SchemeNorm][ColBg].pixel;
      XChangeWindowAttributes(dpy, c->win, CWBackPixel, &swa);
      sendevent(c->win, netatom[Xembed], StructureNotifyMask, CurrentTime,
                XEMBED_EMBEDDED_NOTIFY, 0, systray->win,
                XEMBED_EMBEDDED_VERSION);
      /* FIXME not sure if I have to send these events, too */
      sendevent(c->win, netatom[Xembed], StructureNotifyMask, CurrentTime,
                XEMBED_FOCUS_IN, 0, systray->win, XEMBED_EMBEDDED_VERSION);
      sendevent(c->win, netatom[Xembed], StructureNotifyMask, CurrentTime,
                XEMBED_WINDOW_ACTIVATE, 0, systray->win,
                XEMBED_EMBEDDED_VERSION);
      sendevent(c->win, netatom[Xembed], StructureNotifyMask, CurrentTime,
                XEMBED_MODALITY_ON, 0, systray->win, XEMBED_EMBEDDED_VERSION);
      XSync(dpy, False);
      resizebarwin(selmon);
      updatesystray();
      setclientstate(c, NormalState);
    }
    return;
  }
  if (!c) return;
  if (cme->message_type == netatom[NetWMState]) {
    if (cme->data.l[1] == netatom[NetWMFullscreen] ||
        cme->data.l[2] == netatom[NetWMFullscreen])
      setfullscreen(c, (cme->data.l[0] == 1 /* _NET_WM_STATE_ADD    */
                        || (cme->data.l[0] == 2 /* _NET_WM_STATE_TOGGLE */ &&
                            !c->isfullscreen)));
  } else if (cme->message_type == netatom[NetActiveWindow]) {
    if (c != selmon->sel && !c->isurgent) seturgent(c, 1);
  }
}

void configurenotify(XEvent* e) {
  Monitor* m;
  Client* c;
  XConfigureEvent* ev = &e->xconfigure;
  int dirty;

  /* TODO: updategeom handling sucks, needs to be simplified */
  if (ev->window == root) {
    dirty = (sw != ev->width || sh != ev->height);
    sw = ev->width;
    sh = ev->height;
    if (updategeom() || dirty) {
      drw_resize(drw, sw, bh);
      updatebars();
      for (m = mons; m; m = m->next) {
        for (c = m->clients; c; c = c->next)
          if (c->isfullscreen) resizeclient(c, m->mx, m->my, m->mw, m->mh);
        resizebarwin(m);
      }
      focus(NULL);
      arrange(NULL);
    }
  }
}

void configurerequest(XEvent* e) {
  Client* c;
  Monitor* m;
  XConfigureRequestEvent* ev = &e->xconfigurerequest;
  XWindowChanges wc;

  if ((c = wintoclient(ev->window))) {
    if (ev->value_mask & CWBorderWidth)
      c->bw = ev->border_width;
    else if (c->isfloating) {
      m = c->mon;
      if (ev->value_mask & CWX) {
        c->oldx = c->x;
        c->x = m->mx + ev->x;
      }
      if (ev->value_mask & CWY) {
        c->oldy = c->y;
        c->y = m->my + ev->y;
      }
      if (ev->value_mask & CWWidth) {
        c->oldw = c->w;
        c->w = ev->width;
      }
      if (ev->value_mask & CWHeight) {
        c->oldh = c->h;
        c->h = ev->height;
      }
      if ((c->x + c->w) > m->mx + m->mw && c->isfloating)
        c->x = m->mx + (m->mw / 2 - WIDTH(c) / 2); /* center in x direction */
      if ((c->y + c->h) > m->my + m->mh && c->isfloating)
        c->y = m->my + (m->mh / 2 - HEIGHT(c) / 2); /* center in y direction */
      if ((ev->value_mask & (CWX | CWY)) &&
          !(ev->value_mask & (CWWidth | CWHeight)))
        configure(c);
      if (ISVISIBLE(c)) XMoveResizeWindow(dpy, c->win, c->x, c->y, c->w, c->h);
    } else
      configure(c);
  } else {
    wc.x = ev->x;
    wc.y = ev->y;
    wc.width = ev->width;
    wc.height = ev->height;
    wc.border_width = ev->border_width;
    wc.sibling = ev->above;
    wc.stack_mode = ev->detail;
    XConfigureWindow(dpy, ev->window, ev->value_mask, &wc);
  }
  XSync(dpy, False);
}

void destroynotify(XEvent* e) {
  Client* c;
  XDestroyWindowEvent* ev = &e->xdestroywindow;

  if ((c = wintoclient(ev->window)))
    unmanage(c, 1);
  else if ((c = wintosystrayicon(ev->window))) {
    removesystrayicon(c);
    resizebarwin(selmon);
    updatesystray();
  }
}

void enternotify(XEvent* e) {
  Client* c;
  Monitor* m;
  XCrossingEvent* ev = &e->xcrossing;

  if ((ev->mode != NotifyNormal || ev->detail == NotifyInferior) &&
      ev->window != root)
    return;
  c = wintoclient(ev->window);
  m = c ? c->mon : wintomon(ev->window);
  if (m != selmon) {
    unfocus(selmon->sel, 1);
    selmon = m;
  } else if (!c || c == selmon->sel)
    return;
  focus(c);
}

void expose(XEvent* e) {
  Monitor* m;
  XExposeEvent* ev = &e->xexpose;

  if (ev->count == 0 && (m = wintomon(ev->window))) {
    drawbar(m);
    drawtab(m);
    if (m == selmon) updatesystray();
  }
}

/* there are some broken focus acquiring clients needing extra handling */
void focusin(XEvent* e) {
  XFocusChangeEvent* ev = &e->xfocus;

  if (selmon->sel && ev->window != selmon->sel->win) setfocus(selmon->sel);
}

void keypress(XEvent* e) {
  unsigned int i;
  KeySym keysym;
  XKeyEvent* ev;
  ev = &e->xkey;
  keysym = XKeycodeToKeysym(dpy, (KeyCode)ev->keycode, 0);

  for (i = 0; i < dkeys_len; i++)
    if (keysym == dkeys[i].keysym && CLEANMASK(dkeys[i].mod) == CLEANMASK(ev->state)) {
      srwm_handle_key(dkeys[i].id);
    }
}

void mappingnotify(XEvent* e) {
  XMappingEvent* ev = &e->xmapping;

  XRefreshKeyboardMapping(ev);
  if (ev->request == MappingKeyboard) grabkeys();
}

void maprequest(XEvent* e) {
  static XWindowAttributes wa;
  XMapRequestEvent* ev = &e->xmaprequest;
  Client* i;
  if ((i = wintosystrayicon(ev->window))) {
    sendevent(i->win, netatom[Xembed], StructureNotifyMask, CurrentTime,
              XEMBED_WINDOW_ACTIVATE, 0, systray->win, XEMBED_EMBEDDED_VERSION);
    resizebarwin(selmon);
    updatesystray();
  }

  if (!XGetWindowAttributes(dpy, ev->window, &wa) || wa.override_redirect)
    return;
  if (!wintoclient(ev->window)) manage(ev->window, &wa);
}

void motionnotify(XEvent* e) {
  static Monitor* mon = NULL;
  Monitor* m;
  XMotionEvent* ev = &e->xmotion;

  if (ev->window != root) return;
  if ((m = recttomon(ev->x_root, ev->y_root, 1, 1)) != mon && mon) {
    unfocus(selmon->sel, 1);
    selmon = m;
    focus(NULL);
  }
  mon = m;
}

void propertynotify(XEvent* e) {
  Client* c;
  Window trans;
  XPropertyEvent* ev = &e->xproperty;

  if ((c = wintosystrayicon(ev->window))) {
    if (ev->atom == XA_WM_NORMAL_HINTS) {
      updatesizehints(c);
      updatesystrayicongeom(c, c->w, c->h);
    } else
      updatesystrayiconstate(c, ev);
    resizebarwin(selmon);
    updatesystray();
  }
  if ((ev->window == root) && (ev->atom == XA_WM_NAME))
    updatestatus();
  else if (ev->state == PropertyDelete)
    return; /* ignore */
  else if ((c = wintoclient(ev->window))) {
    switch (ev->atom) {
      default:
        break;
      case XA_WM_TRANSIENT_FOR:
        if (!c->isfloating && (XGetTransientForHint(dpy, c->win, &trans)) &&
            (c->isfloating = (wintoclient(trans)) != NULL))
          arrange(c->mon);
        break;
      case XA_WM_NORMAL_HINTS:
        c->hintsvalid = 0;
        break;
      case XA_WM_HINTS:
        updatewmhints(c);
        drawbars();
        drawtabs();
        break;
    }
    if (ev->atom == XA_WM_NAME || ev->atom == netatom[NetWMName]) {
      updatetitle(c);
      if (c == c->mon->sel) drawbar(c->mon);
      drawtab(c->mon);
    }

    else if (ev->atom == netatom[NetWMIcon]) {
      updateicon(c);
      if (c == c->mon->sel) drawbar(c->mon);
    }

    if (ev->atom == netatom[NetWMWindowType]) updatewindowtype(c);
  }
}

void resizerequest(XEvent* e) {
  XResizeRequestEvent* ev = &e->xresizerequest;
  Client* i;

  if ((i = wintosystrayicon(ev->window))) {
    updatesystrayicongeom(i, ev->width, ev->height);
    resizebarwin(selmon);
    updatesystray();
  }
}

void unmapnotify(XEvent* e) {
  Client* c;
  XUnmapEvent* ev = &e->xunmap;

  if ((c = wintoclient(ev->window))) {
    if (ev->send_event)
      setclientstate(c, WithdrawnState);
    else if (c->ismapped)
      unmanage(c, 0);
  } else if ((c = wintosystrayicon(ev->window))) {
    /* KLUDGE! sometimes icons occasionally unmap their windows, but do
     * _not_ destroy them. We map those windows back */
    XMapRaised(dpy, c->win);
    updatesystray();
  }
}
