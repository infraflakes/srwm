#include "wm.h"

void (*handler[LASTEvent])(XEvent *) = {[ButtonPress] = buttonpress,
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
                                        [UnmapNotify] = unmapnotify};


void buttonpress(XEvent *e) {
  unsigned int i, x, click;
  int loop;
  Arg arg = {0};
  Client *c;
  Monitor *m;
  XButtonPressedEvent *ev = &e->xbutton;

  click = ClkRootWin;
  /* focus monitor if necessary */
  if ((m = wintomon(ev->window)) && m != selmon) {
    unfocus(selmon->sel, 1);
    selmon = m;
    focus(NULL);
  }


  if (ev->window == selmon->tabwin) {
    i = 0;
    x = 0;
    for (int j = 0; j < selmon->ntabs; j++) {
      x += selmon->tab_widths[j];
      if (ev->x > x)
        ++i;
      else
        break;
    }
    if (i < selmon->ntabs) {
      click = ClkTabBar;
      arg.i = i;
    } else {
        x = selmon->ww;
      for (loop = 2; loop >= 0; loop--) {
        x -= selmon->tab_btn_w[loop];
        if (ev->x > x)
          break;
      }
      if (ev->x >= x)
        click = ClkTabPrev + loop;
    }
  } else if ((c = wintoclient(ev->window))) {
    focus(c);
    restack(selmon);
    XAllowEvents(dpy, ReplayPointer, CurrentTime);
    click = ClkClientWin;
  }

execute_handler:

  for (i = 0; i < buttons_len; i++)
    if (click == buttons[i].click && buttons[i].func &&
        buttons[i].button == ev->button &&
        CLEANMASK(buttons[i].mask) == CLEANMASK(ev->state))
       buttons[i].func(
            (click == ClkTabBar && buttons[i].arg.i == 0)
               ? &arg
               : &buttons[i].arg);

  for (i = 0; i < (unsigned int)dbuttons_len; i++)
    if (click == dbuttons[i].click && dbuttons[i].button == ev->button &&
        CLEANMASK(dbuttons[i].mod) == CLEANMASK(ev->state))
      srwm_handle_mouse(dbuttons[i].id);
}

void clientmessage(XEvent *e) {
  Client *c = wintoclient(cme->window);

  if (!c)
    return;
  if (cme->message_type == netatom[NetWMState]) {
    if (cme->data.l[1] == netatom[NetWMFullscreen] ||
        cme->data.l[2] == netatom[NetWMFullscreen])
      setfullscreen(c, (cme->data.l[0] == 1 /* _NET_WM_STATE_ADD    */
                        || (cme->data.l[0] == 2 /* _NET_WM_STATE_TOGGLE */ &&
                            !c->isfullscreen)));
  } else if (cme->message_type == netatom[NetActiveWindow]) {
    if (c != selmon->sel && !c->isurgent)
      seturgent(c, 1);
  }
}

void configurenotify(XEvent *e) {
  Monitor *m;
  Client *c;
  XConfigureEvent *ev = &e->xconfigure;
  int dirty;

  /* TODO: updategeom handling sucks, needs to be simplified */
  if (ev->window == root) {
    dirty = (sw != ev->width || sh != ev->height);
    sw = ev->width;
    sh = ev->height;
    if (updategeom() || dirty) {
      drw_resize(drw, sw, th > 0 ? th : 20);
      for (m = mons; m; m = m->next) {
        for (c = m->clients; c; c = c->next)
          if (c->isfullscreen)
            resizeclient(c, m->mx, m->my, m->mw, m->mh);
      }
      focus(NULL);
      arrange(NULL);
    }
  }
}

void configurerequest(XEvent *e) {
  Client *c;
  Monitor *m;
  XConfigureRequestEvent *ev = &e->xconfigurerequest;
  XWindowChanges wc;

  if ((c = wintoclient(ev->window))) {
    if (ev->value_mask & CWBorderWidth)
      c->bw = ev->border_width;
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
    if ((c->x + c->w) > m->mx + m->mw)
      c->x = m->mx + (m->mw / 2 - WIDTH(c) / 2); /* center in x direction */
    if ((c->y + c->h) > m->my + m->mh)
      c->y = m->my + (m->mh / 2 - HEIGHT(c) / 2); /* center in y direction */
    if ((ev->value_mask & (CWX | CWY)) &&
        !(ev->value_mask & (CWWidth | CWHeight)))
      configure(c);
    if (ISVISIBLE(c))
      XMoveResizeWindow(dpy, c->win, c->x, c->y, c->w, c->h);
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

void destroynotify(XEvent *e) {
  Client *c;
  XDestroyWindowEvent *ev = &e->xdestroywindow;

  if ((c = wintoclient(ev->window)))
    unmanage(c, 1);
}

void enternotify(XEvent *e) {
  Client *c;
  Monitor *m;
  XCrossingEvent *ev = &e->xcrossing;

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

void expose(XEvent *e) {
  Monitor *m;
  XExposeEvent *ev = &e->xexpose;

  if (ev->count == 0 && (m = wintomon(ev->window))) {
    drawtab(m);
  }
}

/* there are some broken focus acquiring clients needing extra handling */
void focusin(XEvent *e) {
  XFocusChangeEvent *ev = &e->xfocus;

  if (selmon->sel && ev->window != selmon->sel->win)
    setfocus(selmon->sel);
}

void keypress(XEvent *e) {
  unsigned int i;
  KeySym keysym;
  XKeyEvent *ev;
  ev = &e->xkey;
  keysym = XKeycodeToKeysym(dpy, (KeyCode)ev->keycode, 0);

  for (i = 0; i < dkeys_len; i++)
    if (keysym == dkeys[i].keysym &&
        CLEANMASK(dkeys[i].mod) == CLEANMASK(ev->state)) {
      srwm_handle_key(dkeys[i].id);
    }
}

void mappingnotify(XEvent *e) {
  XMappingEvent *ev = &e->xmapping;

  XRefreshKeyboardMapping(ev);
  if (ev->request == MappingKeyboard)
    grabkeys();
}

void maprequest(XEvent *e) {
  if (!XGetWindowAttributes(dpy, ev->window, &wa) || wa.override_redirect)
    return;
  if (!wintoclient(ev->window))
    manage(ev->window, &wa);
}

void motionnotify(XEvent *e) {
  static Monitor *mon = NULL;
  Monitor *m;
  XMotionEvent *ev = &e->xmotion;

  // Edge auto-pan should work regardless of which window the cursor is over
  if (selmon && selmon->canvas_zoom < 1.0f) {
    canvas_edge_autopan(ev->x_root, ev->y_root, NULL, NULL, NULL);
  }

  if (ev->window != root)
    return;
  if ((m = recttomon(ev->x_root, ev->y_root, 1, 1)) != mon && mon) {
    unfocus(selmon->sel, 1);
    selmon = m;
    focus(NULL);
  }
  mon = m;
}

void propertynotify(XEvent *e) {
  Client *c;
  XPropertyEvent *ev = &e->xproperty;

  if ((ev->window == root) && (ev->atom == XA_WM_NAME))
    updatestatus();
  else if (ev->state == PropertyDelete)
    return; /* ignore */
  else if ((c = wintoclient(ev->window))) {
    switch (ev->atom) {
    default:
      break;
    case XA_WM_HINTS:
      updatewmhints(c);
      drawtabs();
      break;
    }
    if (ev->atom == XA_WM_NAME || ev->atom == netatom[NetWMName]) {
      updatetitle(c);
      if (c == c->mon->sel)
        drawtab(c->mon);
      drawtab(c->mon);
    }

    else if (ev->atom == netatom[NetWMIcon]) {
      updateicon(c);
      if (c == c->mon->sel)
        drawtab(c->mon);
    }

    if (ev->atom == netatom[NetWMWindowType])
      updatewindowtype(c);
  }
}

void resizerequest(XEvent *e) {
  /* was systray-only, now unused */
}

void unmapnotify(XEvent *e) {
  Client *c;
  XUnmapEvent *ev = &e->xunmap;

  if ((c = wintoclient(ev->window))) {
    if (ev->send_event)
      setclientstate(c, WithdrawnState);
    else if (c->ismapped)
      unmanage(c, 0);
  }
}
