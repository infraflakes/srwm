#include "wm.h"
#include <sys/select.h>

void movemouse(const Arg* arg) {
  int x, y, ocx, ocy, nx, ny;
  Client* c;
  Monitor* m;
  XEvent ev;
  Time lasttime = 0;

  if (!(c = selmon->sel)) return;
  if (c->isfullscreen) /* no support moving fullscreen windows by mouse */
    return;
  restack(selmon);
  ocx = c->x;
  ocy = c->y;
  if (XGrabPointer(dpy, root, False, MOUSEMASK, GrabModeAsync, GrabModeAsync,
                   None, cursor[CurMove]->cursor, CurrentTime) != GrabSuccess)
    return;
  if (!getrootptr(&x, &y)) return;
  int x11_fd = ConnectionNumber(dpy);
  int got_release = 0;
  while (!got_release) {
    int need_autopan = selmon->canvas_zoom < 1.0f;

    if (need_autopan) {
        // Non-blocking check for matching events
        if (!XCheckMaskEvent(dpy, MOUSEMASK | ExposureMask | SubstructureRedirectMask, &ev)) {
            // No matching event available — wait with timeout for continuous auto-pan
            fd_set fds;
            FD_ZERO(&fds);
            FD_SET(x11_fd, &fds);
            struct timeval tv = { .tv_sec = 0, .tv_usec = 16000 };
            int ret = select(x11_fd + 1, &fds, NULL, NULL, &tv);

            if (ret == 0) {
                // Timeout — auto-pan if cursor is at edge
                int cx, cy;
                Window dummy_w;
                int di;
                unsigned int dui;
                if (XQueryPointer(dpy, root, &dummy_w, &dummy_w, &cx, &cy, &di, &di, &dui)) {
                    canvas_edge_autopan(cx, cy, c, NULL, NULL);
                // Don't adjust ocx/ocy — the dragged window stays under the cursor
                // while the canvas (other windows) slides underneath
                }
            }
            // If ret > 0, loop back and XCheckMaskEvent will find the new event
            continue;
        }
        // If XCheckMaskEvent returned true, ev is populated — fall through to process it
    } else {
        // No auto-pan needed — use blocking wait (original behavior)
        XMaskEvent(dpy, MOUSEMASK | ExposureMask | SubstructureRedirectMask, &ev);
    }

    switch (ev.type) {
      case ConfigureRequest:
      case Expose:
      case MapRequest:
        handler[ev.type](&ev);
        break;
      case MotionNotify:
        if ((ev.xmotion.time - lasttime) <= (1000 / 120)) continue;
        lasttime = ev.xmotion.time;

        // --- Edge auto-pan when zoomed out in canvas mode ---
        canvas_edge_autopan(ev.xmotion.x_root, ev.xmotion.y_root, c, NULL, NULL);

        nx = ocx + (ev.xmotion.x - x);
        ny = ocy + (ev.xmotion.y - y);

        resize(c, nx, ny, c->w, c->h, 1);
        break;
      case ButtonRelease:
        got_release = 1;
        break;
    }
}
  XUngrabPointer(dpy, CurrentTime);
  if ((m = recttomon(c->x, c->y, c->w, c->h)) != selmon) {
    sendmon(c, m);
    selmon = m;
    focus(NULL);
  }
}

void resizemouse(const Arg* arg) {
  int ocx, ocy, nw, nh;
  Client* c;
  Monitor* m;
  XEvent ev;
  Time lasttime = 0;

  if (!(c = selmon->sel)) return;
  if (c->isfullscreen) /* no support resizing fullscreen windows by mouse */
    return;
  restack(selmon);
  ocx = c->x;
  ocy = c->y;
  if (XGrabPointer(dpy, root, False, MOUSEMASK, GrabModeAsync, GrabModeAsync,
                   None, cursor[CurResize]->cursor, CurrentTime) != GrabSuccess)
    return;
  XWarpPointer(dpy, None, c->win, 0, 0, 0, 0, c->w + c->bw - 1,
               c->h + c->bw - 1);
  int x11_fd = ConnectionNumber(dpy);
  int got_release = 0;
  while (!got_release) {
    int need_autopan = selmon->canvas_zoom < 1.0f;

    if (need_autopan) {
        if (!XCheckMaskEvent(dpy, MOUSEMASK | ExposureMask | SubstructureRedirectMask, &ev)) {
            fd_set fds;
            FD_ZERO(&fds);
            FD_SET(x11_fd, &fds);
            struct timeval tv = { .tv_sec = 0, .tv_usec = 16000 };
            int ret = select(x11_fd + 1, &fds, NULL, NULL, &tv);

            if (ret == 0) {
                int cx, cy;
                Window dummy_w;
                int di;
                unsigned int dui;
                if (XQueryPointer(dpy, root, &dummy_w, &dummy_w, &cx, &cy, &di, &di, &dui)) {
                    int pan_dx = 0, pan_dy = 0;
                    canvas_edge_autopan(cx, cy, c, &pan_dx, &pan_dy);
                }
            }
            continue;
        }
    } else {
        XMaskEvent(dpy, MOUSEMASK | ExposureMask | SubstructureRedirectMask, &ev);
    }

    switch (ev.type) {
      case ConfigureRequest:
      case Expose:
      case MapRequest:
        handler[ev.type](&ev);
        break;
      case MotionNotify:
        if ((ev.xmotion.time - lasttime) <= (1000 / 60)) continue;
        lasttime = ev.xmotion.time;

        canvas_edge_autopan(ev.xmotion.x_root, ev.xmotion.y_root, c, NULL, NULL);

        nw = MAX(ev.xmotion.x - ocx - 2 * c->bw + 1, 1);
        nh = MAX(ev.xmotion.y - ocy - 2 * c->bw + 1, 1);

        resize(c, c->x, c->y, nw, nh, 1);
        break;
      case ButtonRelease:
        got_release = 1;
        break;
    }
}
  XWarpPointer(dpy, None, c->win, 0, 0, 0, 0, c->w + c->bw - 1,
               c->h + c->bw - 1);
  XUngrabPointer(dpy, CurrentTime);
  while (XCheckMaskEvent(dpy, EnterWindowMask, &ev));
  if ((m = recttomon(c->x, c->y, c->w, c->h)) != selmon) {
    sendmon(c, m);
    selmon = m;
    focus(NULL);
  }
}
