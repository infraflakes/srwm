#include "wm.h"

void drawtabs(void);
void drawtab(Monitor* m);

void updatebars(void) {
  Monitor* m;
  XSetWindowAttributes wa = {
      .override_redirect = True,
      .background_pixmap = ParentRelative,
      .event_mask = ButtonPressMask | ExposureMask | PointerMotionMask};
  XClassHint ch = {"srwm", "srwm"};
  for (m = mons; m; m = m->next) {
    if (m->tabwin) continue;
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
  }
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

int cmpint(const void* p1, const void* p2) {
  return *(const int*)p1 - *(const int*)p2;
}

void drawtabs(void) {
  Monitor* m;

  for (m = mons; m; m = m->next) drawtab(m);
}

void drawtab(Monitor* m) {
  if (th <= 0) return;
  Client* c;
  int i;

  if (m->ntabs == 0) {
    XUnmapWindow(dpy, m->tabwin);
    return;
  }
  XMapWindow(dpy, m->tabwin);

  char* btn_prev = "\uf804";
  char* btn_next = "\uf805";
  char* btn_close = "\uf6a7 ";
  int buttons_w = 0;
  int sorted_label_widths[MAXTABS];
  int tot_width = 0;
  int maxsize = th;
  int x = 0;
  int w = 0;
  int mw = m->ww;
  buttons_w += TEXTW(btn_prev) - lrpad + tab_tile_inner_padding_horizontal;
  buttons_w += TEXTW(btn_next) - lrpad + tab_tile_inner_padding_horizontal;
  buttons_w += TEXTW(btn_close) - lrpad + tab_tile_inner_padding_horizontal;
  tot_width = buttons_w;

  for (i = 0; i < m->ntabs; i++) {
    c = m->tab_order[i];
    m->tab_widths[i] =
        MIN(TEXTW(c->name) - lrpad + tab_tile_outer_padding_horizontal + tab_tile_inner_padding_horizontal, 250);
    tot_width += m->tab_widths[i];
  }

  if (tot_width > mw) {
    memcpy(sorted_label_widths, m->tab_widths, sizeof(int) * m->ntabs);
    qsort(sorted_label_widths, m->ntabs, sizeof(int), cmpint);
    tot_width = buttons_w;
    for (i = 0; i < m->ntabs; ++i) {
      if (tot_width + (m->ntabs - i) * sorted_label_widths[i] > mw) break;
      tot_width += sorted_label_widths[i];
    }
    maxsize = (m->ntabs - i > 0) ? (mw - tot_width) / (m->ntabs - i) : mw;
    if (maxsize < 1) maxsize = 1;
  } else {
    maxsize = mw;
  }
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
