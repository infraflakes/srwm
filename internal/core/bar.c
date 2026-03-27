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