#include "wm.h"
#include <sys/select.h>

void moveorplace(const Arg* arg) {
  if (selmon->sel && selmon->sel->isfloating)
    movemouse(arg);
  else
    placemouse(arg);
}

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
    int need_autopan = selmon->canvas_mode &&
                       selmon->canvas[getcurrenttag(selmon)].zoom < 1.0f;

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

        // Disable edge snapping when zoomed out in canvas mode
        if (!(selmon->canvas_mode && selmon->canvas[getcurrenttag(selmon)].zoom < 1.0f)) {
          if (abs(selmon->wx - nx) < px_till_snapping_to_screen_edge)
            nx = selmon->wx;
          else if (abs((selmon->wx + selmon->ww) - (nx + WIDTH(c))) < px_till_snapping_to_screen_edge)
            nx = selmon->wx + selmon->ww - WIDTH(c);
          if (abs(selmon->wy - ny) < px_till_snapping_to_screen_edge)
            ny = selmon->wy;
          else if (abs((selmon->wy + selmon->wh) - (ny + HEIGHT(c))) < px_till_snapping_to_screen_edge)
            ny = selmon->wy + selmon->wh - HEIGHT(c);
        }
        if (!c->isfloating && (abs(nx - c->x) > px_till_snapping_to_screen_edge || abs(ny - c->y) > px_till_snapping_to_screen_edge))
          togglefloating(NULL);
        if (c->isfloating) resize(c, nx, ny, c->w, c->h, 1);
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

void placemouse(const Arg* arg) {
  int x, y, px, py, ocx, ocy, nx = -9999, ny = -9999, freemove = 0;
  Client *c, *r = NULL, *at, *prevr;
  Monitor* m;
  XEvent ev;
  XWindowAttributes wa;
  Time lasttime = 0;
  int attachmode, prevattachmode;
  attachmode = prevattachmode = -1;

  if (!(c = selmon->sel)) return;
  if (c->isfullscreen) /* no support placing fullscreen windows by mouse */
    return;
  restack(selmon);
  prevr = c;
  if (XGrabPointer(dpy, root, False, MOUSEMASK, GrabModeAsync, GrabModeAsync,
                   None, cursor[CurMove]->cursor, CurrentTime) != GrabSuccess)
    return;

  c->isfloating = 0;
  c->beingmoved = 1;

  XGetWindowAttributes(dpy, c->win, &wa);
  ocx = wa.x;
  ocy = wa.y;

  if (arg->i == 2)  // warp cursor to client center
    XWarpPointer(dpy, None, c->win, 0, 0, 0, 0, WIDTH(c) / 2, HEIGHT(c) / 2);

  if (!getrootptr(&x, &y)) return;

  do {
    XMaskEvent(dpy, MOUSEMASK | ExposureMask | SubstructureRedirectMask, &ev);
    switch (ev.type) {
      case ConfigureRequest:
      case Expose:
      case MapRequest:
        handler[ev.type](&ev);
        break;
      case MotionNotify:
        if ((ev.xmotion.time - lasttime) <= (1000 / 60)) continue;
        lasttime = ev.xmotion.time;

        nx = ocx + (ev.xmotion.x - x);
        ny = ocy + (ev.xmotion.y - y);

        if (!freemove && (abs(nx - ocx) > px_till_snapping_to_screen_edge || abs(ny - ocy) > px_till_snapping_to_screen_edge))
          freemove = 1;

        if (freemove) XMoveWindow(dpy, c->win, nx, ny);

        if ((m = recttomon(ev.xmotion.x, ev.xmotion.y, 1, 1)) && m != selmon)
          selmon = m;

        if (arg->i == 1) {  // tiled position is relative to the client window
                            // center point
          px = nx + wa.width / 2;
          py = ny + wa.height / 2;
        } else {  // tiled position is relative to the mouse cursor
          px = ev.xmotion.x;
          py = ev.xmotion.y;
        }

        r = recttoclient(px, py, 1, 1);

        if (!r || r == c) break;

        attachmode = 0;  // below
        if (((float)(r->y + r->h - py) / r->h) >
            ((float)(r->x + r->w - px) / r->w)) {
          if (abs(r->y - py) < r->h / 2) attachmode = 1;  // above
        } else if (abs(r->x - px) < r->w / 2)
          attachmode = 1;  // above

        if ((r && r != prevr) || (attachmode != prevattachmode)) {
          detachstack(c);
          detach(c);
          if (c->mon != r->mon) {
            arrangemon(c->mon);
            c->tags = r->mon->tagset[r->mon->seltags];
          }

          c->mon = r->mon;
          r->mon->sel = r;

          if (attachmode) {
            if (r == r->mon->clients)
              attach(c);
            else {
              for (at = r->mon->clients; at->next != r; at = at->next);
              c->next = at->next;
              at->next = c;
            }
          } else {
            c->next = r->next;
            r->next = c;
          }

          attachstack(c);
          arrangemon(r->mon);
          prevr = r;
          prevattachmode = attachmode;
        }
        break;
    }
  } while (ev.type != ButtonRelease);
  XUngrabPointer(dpy, CurrentTime);

  if ((m = recttomon(ev.xmotion.x, ev.xmotion.y, 1, 1)) && m != c->mon) {
    detach(c);
    detachstack(c);
    arrangemon(c->mon);
    c->mon = m;
    c->tags = m->tagset[m->seltags];
    attach(c);
    attachstack(c);
    selmon = m;
  }

  focus(c);
  c->beingmoved = 0;

  if (nx != -9999) resize(c, nx, ny, c->w, c->h, 0);
  arrangemon(c->mon);
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
    int need_autopan = selmon->canvas_mode &&
                       selmon->canvas[getcurrenttag(selmon)].zoom < 1.0f;

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

        if (c->mon->wx + nw >= selmon->wx &&
            c->mon->wx + nw <= selmon->wx + selmon->ww &&
            c->mon->wy + nh >= selmon->wy &&
            c->mon->wy + nh <= selmon->wy + selmon->wh) {
          if (!c->isfloating &&
              (abs(nw - c->w) > px_till_snapping_to_screen_edge || abs(nh - c->h) > px_till_snapping_to_screen_edge))
            togglefloating(NULL);
        }
        if (c->isfloating) resize(c, c->x, c->y, nw, nh, 1);
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
