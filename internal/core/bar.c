#include "wm.h"

int drawstatusbar(Monitor* m, int bh, char* stext) {
  int ret, i, w, x, len;
  short isCode = 0;
  char* text;
  char* p;

  len = strlen(stext) + 1;
  if (!(text = (char*)malloc(sizeof(char) * len))) die("malloc");
  p = text;
  memcpy(text, stext, len);

  /* compute width of the status text */
  w = 0;
  i = -1;
  while (text[++i]) {
    if (text[i] == '^') {
      if (!isCode) {
        isCode = 1;
        text[i] = '\0';
        w += TEXTW(text) - lrpad;
        text[i] = '^';
        if (text[++i] == 'f') w += atoi(text + ++i);
      } else {
        isCode = 0;
        text = text + i + 1;
        i = -1;
      }
    }
  }
  if (!isCode)
    w += TEXTW(text) - lrpad;
  else
    isCode = 0;
  text = p;

  w += bar_horizontal_padding;
  ret = x = m->ww - w;
  x = m->ww - w - getsystraywidth();

  drw_setscheme(drw, scheme[LENGTH(colors)]);
  drw->scheme[ColFg] = scheme[SchemeNorm][ColFg];
  drw->scheme[ColBg] = bar_bg;
  drw_rect(drw, x, 0, w, bh, 1, 1);
  x += bar_horizontal_padding / 2;

  /* process status text */
  i = -1;
  while (text[++i]) {
    if (text[i] == '^' && !isCode) {
      isCode = 1;

      text[i] = '\0';
      w = TEXTW(text) - lrpad;
      drw_text(drw, x, bar_vertical_padding / 2, w, bh - bar_vertical_padding, 0, text, 0);

      x += w;

      /* process code */
      while (text[++i] != '^') {
        if (text[i] == 'c') {
          char buf[8];
          memcpy(buf, (char*)text + i + 1, 7);
          buf[7] = '\0';
          drw_clr_create(drw, &drw->scheme[ColFg], buf);
          i += 7;
        } else if (text[i] == 'b') {
          char buf[8];
          memcpy(buf, (char*)text + i + 1, 7);
          buf[7] = '\0';
          drw_clr_create(drw, &drw->scheme[ColBg], buf);
          i += 7;
         } else if (text[i] == 'd') {
           drw->scheme[ColFg] = scheme[SchemeNorm][ColFg];
           drw->scheme[ColBg] = bar_bg;
         } else if (text[i] == 'r') {
          int rx = atoi(text + ++i);
          while (text[++i] != ',');
          int ry = atoi(text + ++i);
          while (text[++i] != ',');
          int rw = atoi(text + ++i);
          while (text[++i] != ',');
          int rh = atoi(text + ++i);

          drw_rect(drw, rx + x, ry + bar_vertical_padding / 2, rw, rh, 1, 0);
        } else if (text[i] == 'f') {
          x += atoi(text + ++i);
        }
      }

      text = text + i + 1;
      i = -1;
      isCode = 0;
    }
  }

  if (!isCode) {
    w = TEXTW(text) - lrpad;
    drw_text(drw, x, bar_vertical_padding / 2, w, bh - bar_vertical_padding, 0, text, 0);
  }

  drw_setscheme(drw, scheme[SchemeNorm]);
  free(p);

  return ret;
}


void drawbar(Monitor* m) {
  int x, y = 0, w, stw = 0;
  int bh_n = bh;
  unsigned int i;
  int occ[9] = {0}, urg[9] = {0};

  XSetForeground(drw->dpy, drw->gc, bar_bg.pixel);
  XFillRectangle(drw->dpy, drw->drawable, drw->gc, 0, 0, m->ww, bh);

  if (systray_enable && m == systraytomon(m)) stw = getsystraywidth();

  if (!m->showbar) return;

  /* draw status first so it can be overdrawn by workspaces later */
  int sbar_x = 0;
  if (m == selmon) {
    sbar_x = drawstatusbar(m, bh_n, stext);
  }

  resizebarwin(m);
  for (Client *c = m->clients; c; c = c->next) {
    occ[c->ws] = 1;
    if (c->isurgent) urg[c->ws] = 1;
  }
  x = 0;
  for (i = 0; i < WS_COUNT; i++) {
    w = TEXTW(ws_labels[i]);
    int use_colorful = m->colorful_ws && (!ws_colorful_occupied_only || occ[i]);
    drw_setscheme(
        drw, scheme[use_colorful ? ws_schemes[i] : (occ[i] ? SchemeSel : SchemeWs)]);
    drw_text(drw, x, y, w, bh_n, lrpad / 2, ws_labels[i], urg[i]);
    if (ws_underline_for_all || (int)i == m->current_ws)
      drw_rect(drw, x + ws_underline_padding, bh_n - ws_underline_size - ws_underline_offset_from_bar_bottom,
               w - (ws_underline_padding * 2), ws_underline_size, 1, 0);
    x += w;
  }

  // CHANGE TITLE LENGTH
  if (sbar_x > 0)
    w = sbar_x - stw - x;
  else
    w = m->ww - x - stw;
  if (w < 0) w = 0;
  if (w > bh_n) {
    if (m->sel) {
      drw_setscheme(drw, scheme[m == selmon ? SchemeTitle : SchemeNorm]);
      drw_text(drw, x, 0, w, bh,
               lrpad / 2 + (m->sel->icon ? m->sel->icw + ICONSPACING : 0),
               m->sel->name, 0);
      if (m->sel->icon)
        drw_pic(drw, x + lrpad / 2, (bh - m->sel->ich) / 2, m->sel->icw,
                m->sel->ich, m->sel->icon);
    } else {
       XSetForeground(drw->dpy, drw->gc, scheme[SchemeTitle][ColBg].pixel);
       XFillRectangle(drw->dpy, drw->drawable, drw->gc, x, y, w, bh_n);
    }
  }
  drw_map(drw, m->barwin, 0, 0, m->ww - stw, bh);
}

void drawbars(void) {
  Monitor* m;

  for (m = mons; m; m = m->next) drawbar(m);
}

void drawtabs(void) {
  Monitor* m;

  for (m = mons; m; m = m->next) drawtab(m);
}

int cmpint(const void* p1, const void* p2) {
  return *(const int*)p1 - *(const int*)p2;
}

void drawtab(Monitor* m) {
  if (th <= 0) return;
  Client* c;
  int i;

  /* Hide tab bar when no windows, show when there are windows */
  if (m->ntabs == 0) {
    XUnmapWindow(dpy, m->tabwin);
    return;
  }
  XMapWindow(dpy, m->tabwin);

  char* btn_prev = "";
  char* btn_next = "";
  char* btn_close = " ";
  int buttons_w = 0;
  int sorted_label_widths[MAXTABS];
  int tot_width = 0;
  int maxsize = bh;
  int x = 0;
  int w = 0;
  int mw = m->ww;
  buttons_w += TEXTW(btn_prev) - lrpad + tab_tile_inner_padding_horizontal;
  buttons_w += TEXTW(btn_next) - lrpad + tab_tile_inner_padding_horizontal;
  buttons_w += TEXTW(btn_close) - lrpad + tab_tile_inner_padding_horizontal;
  tot_width = buttons_w;

  /* Calculate widths - ntabs is maintained by rebuild_tab_order/manage/unmanage */
  for (i = 0; i < m->ntabs; i++) {
    c = m->tab_order[i];
    m->tab_widths[i] =
        MIN(TEXTW(c->name) - lrpad + tab_tile_outer_padding_horizontal + tab_tile_inner_padding_horizontal, 250);
    tot_width += m->tab_widths[i];
  }

  if (tot_width > mw) {  // not enough space to display the labels, they need to be truncated
    memcpy(sorted_label_widths, m->tab_widths, sizeof(int) * m->ntabs);
    qsort(sorted_label_widths, m->ntabs, sizeof(int), cmpint);
    tot_width = buttons_w;  /* reset — only buttons are "locked in" so far */
    for (i = 0; i < m->ntabs; ++i) {
      if (tot_width + (m->ntabs - i) * sorted_label_widths[i] > mw) break;
      tot_width += sorted_label_widths[i];
    }
    maxsize = (m->ntabs - i > 0) ? (mw - tot_width) / (m->ntabs - i) : mw;
    if (maxsize < 1) maxsize = 1;  /* prevent zero/negative tab widths */
  } else {
    maxsize = mw;
  }
  /* cleans window */
  drw_setscheme(drw, scheme[TabNorm]);
  drw_rect(drw, 0, 0, mw, th, 1, 1);

  for (i = 0; i < m->ntabs; i++) {
    c = m->tab_order[i];
    if (m->tab_widths[i] > maxsize) m->tab_widths[i] = maxsize;
    w = m->tab_widths[i];
    drw_setscheme(drw, scheme[(c == m->sel) ? TabSel : TabNorm]);
    drw_text(drw, x + tab_tile_inner_padding_horizontal / 2, tab_tile_vertical_padding / 2, w - tab_tile_outer_padding_horizontal, th - tab_tile_vertical_padding, tab_tile_outer_padding_horizontal / 2, c->name, 0);
    x += w;
  }

  w = mw - buttons_w - x;
  x += w;
   drw_setscheme(drw, scheme[SchemeBtnPrev]);
   drw->scheme[ColBg] = scheme[TabNorm][ColBg];
   w = TEXTW(btn_prev) - lrpad + tab_tile_inner_padding_horizontal;
   m->tab_btn_w[0] = w;
   drw_text(drw, x + tab_tile_inner_padding_horizontal / 2, tab_tile_vertical_padding / 2, w, th - tab_tile_vertical_padding, 0, btn_prev, 0);
   x += w;
   drw_setscheme(drw, scheme[SchemeBtnNext]);
   drw->scheme[ColBg] = scheme[TabNorm][ColBg];
   w = TEXTW(btn_next) - lrpad + tab_tile_inner_padding_horizontal;
   m->tab_btn_w[1] = w;
   drw_text(drw, x + tab_tile_inner_padding_horizontal / 2, tab_tile_vertical_padding / 2, w, th - tab_tile_vertical_padding, 0, btn_next, 0);
   x += w;
   drw_setscheme(drw, scheme[SchemeBtnClose]);
   drw->scheme[ColBg] = scheme[TabNorm][ColBg];
   w = TEXTW(btn_close) - lrpad + tab_tile_outer_padding_horizontal;
   m->tab_btn_w[2] = w;
   drw_text(drw, x + tab_tile_inner_padding_horizontal / 2, tab_tile_vertical_padding / 2, w, th - tab_tile_vertical_padding, 0, btn_close, 0);
   x += w;
  drw_map(drw, m->tabwin, 0, 0, m->ww, th);
}

void updatebarpos(Monitor* m) {
  m->wy = m->my;
  m->wh = m->mh;

  if (m->showbar) {
    m->wh -= bh;
    m->by = m->topbar ? m->wy : m->wy + m->wh;
    if (m->topbar) m->wy += bh;
  } else {
    m->by = -bh;
  }  
}

void updatebars(void) {
  unsigned int w;
  Monitor* m;
  XSetWindowAttributes wa = {
      .override_redirect = True,
      .background_pixmap = ParentRelative,
      .event_mask = ButtonPressMask | ExposureMask | PointerMotionMask};

  XClassHint ch = {"srwm", "srwm"};
  for (m = mons; m; m = m->next) {
    if (m->barwin) continue;
    w = m->ww;
    if (systray_enable && m == systraytomon(m)) w -= getsystraywidth();
    m->barwin = XCreateWindow(
        dpy, root, m->wx, m->by, w, bh, 0,
        DefaultDepth(dpy, screen), CopyFromParent, DefaultVisual(dpy, screen),
        CWOverrideRedirect | CWBackPixmap | CWEventMask, &wa);
    XDefineCursor(dpy, m->barwin, cursor[CurNormal]->cursor);
    if (systray_enable && m == systraytomon(m)) XMapRaised(dpy, systray->win);
    XMapRaised(dpy, m->barwin);
    XChangeProperty(dpy, m->barwin, netatom[NetWMWindowType], XA_ATOM, 32,
                PropModeReplace, (unsigned char*)&netatom[NetWMWindowTypeDock], 1);
    if (th > 0) {
    m->tabwin = XCreateWindow(
        dpy, root, m->wx, m->ty, m->ww, th, 0,
        DefaultDepth(dpy, screen), CopyFromParent, DefaultVisual(dpy, screen),
        CWOverrideRedirect | CWBackPixmap | CWEventMask, &wa);
    XDefineCursor(dpy, m->tabwin, cursor[CurNormal]->cursor);
    XMapWindow(dpy, m->tabwin);
    XChangeProperty(dpy, m->tabwin, netatom[NetWMWindowType], XA_ATOM, 32,
               PropModeReplace, (unsigned char*)&netatom[NetWMWindowTypeDock], 1);
    XSetClassHint(dpy, m->tabwin, &ch);
    }
    XSetClassHint(dpy, m->barwin, &ch);
  }
}

void resizebarwin(Monitor* m) {
  unsigned int w = m->ww;
  if (systray_enable && m == systraytomon(m)) w -= getsystraywidth();
  XMoveResizeWindow(dpy, m->barwin, m->wx, m->by, w, bh);
}

void updatestatus(void) {
  if (!gettextprop(root, XA_WM_NAME, stext, sizeof(stext)))
    strcpy(stext, "srwm"); //fallback string
  drawbar(selmon);
  updatesystray();
}

uint32_t prealpha(uint32_t p) {
  uint8_t a = p >> 24u;
  uint32_t rb = (a * (p & 0xFF00FFu)) >> 8u;
  uint32_t g = (a * (p & 0x00FF00u)) >> 8u;
  return (rb & 0xFF00FFu) | (g & 0x00FF00u) | (a << 24u);
}

Picture geticonprop(Window win, unsigned int* picw, unsigned int* pich) {
  int format;
  unsigned long n, extra, *p = NULL;
  Atom real;

  if (XGetWindowProperty(dpy, win, netatom[NetWMIcon], 0L, LONG_MAX, False,
                         AnyPropertyType, &real, &format, &n, &extra,
                         (unsigned char**)&p) != Success)
    return None;
  if (n == 0 || format != 32) {
    XFree(p);
    return None;
  }

  unsigned long* bstp = NULL;
  uint32_t w, h, sz;
  {
    unsigned long* i;
    const unsigned long* end = p + n;
    uint32_t bstd = UINT32_MAX, d, m;
    for (i = p; i < end - 1; i += sz) {
      if ((w = *i++) >= 16384 || (h = *i++) >= 16384) {
        XFree(p);
        return None;
      }
      if ((sz = w * h) > end - i) break;
      if ((m = w > h ? w : h) >= ICONSIZE && (d = m - ICONSIZE) < bstd) {
        bstd = d;
        bstp = i;
      }
    }
    if (!bstp) {
      for (i = p; i < end - 1; i += sz) {
        if ((w = *i++) >= 16384 || (h = *i++) >= 16384) {
          XFree(p);
          return None;
        }
        if ((sz = w * h) > end - i) break;
        if ((d = ICONSIZE - (w > h ? w : h)) < bstd) {
          bstd = d;
          bstp = i;
        }
      }
    }
    if (!bstp) {
      XFree(p);
      return None;
    }
  }

  if ((w = *(bstp - 2)) == 0 || (h = *(bstp - 1)) == 0) {
    XFree(p);
    return None;
  }

  uint32_t icw, ich;
  if (w <= h) {
    ich = ICONSIZE;
    icw = w * ICONSIZE / h;
    if (icw == 0) icw = 1;
  } else {
    icw = ICONSIZE;
    ich = h * ICONSIZE / w;
    if (ich == 0) ich = 1;
  }
  *picw = icw;
  *pich = ich;

  uint32_t i, *bstp32 = (uint32_t*)bstp;
  for (sz = w * h, i = 0; i < sz; ++i) bstp32[i] = prealpha(bstp[i]);

  Picture ret = drw_picture_create_resized(drw, (char*)bstp, w, h, icw, ich);
  XFree(p);

  return ret;
}

int next_ws(void) {
  return (selmon->current_ws + 1) % WS_COUNT;
}

int prev_ws(void) {
  return (selmon->current_ws - 1 + WS_COUNT) % WS_COUNT;
}

void ws_to_next(const Arg* arg) {
  if (!selmon->sel) return;
  int ws = next_ws();
  move_to_ws(&(const Arg){.i = ws});
  view(&(const Arg){.i = ws});
}

void ws_to_prev(const Arg* arg) {
  if (!selmon->sel) return;
  int ws = prev_ws();
  move_to_ws(&(const Arg){.i = ws});
  view(&(const Arg){.i = ws});
}
