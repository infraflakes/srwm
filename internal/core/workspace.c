#include "wm.h"

void view(const Arg* arg) {
  int ws = arg->i;
  if (ws == -1) {
    /* swap current and previous workspace */
    int tmp = selmon->current_ws;
    selmon->current_ws = selmon->previous_ws;
    selmon->previous_ws = tmp;
  } else {
    if (ws < 0 || ws >= WS_COUNT) return;
    if (ws == selmon->current_ws) return;
    selmon->previous_ws = selmon->current_ws;
    selmon->current_ws = ws;
  }
  if (selmon->showbar != selmon->showbar_per_ws[selmon->current_ws])
    togglebar(NULL);
  rebuild_tab_order(selmon);
  focus(NULL);
  arrange(selmon);
  publish_canvas_state(selmon);
  updatecurrentdesktop();
}

void move_to_ws(const Arg* arg) {
  int ws = arg->i;
  if (!selmon->sel || ws < 0 || ws >= WS_COUNT) return;
  selmon->sel->ws = ws;
  setclientwsprop(selmon->sel);
  rebuild_tab_order(selmon);
  focus(NULL);
  arrange(selmon);
}

void togglebar(const Arg* arg) {
  selmon->showbar = !selmon->showbar;
  selmon->showbar_per_ws[selmon->current_ws] = selmon->showbar;
  updatebarpos(selmon);
  resizebarwin(selmon);
  if (systray_enable) {
    XWindowChanges wc;
    if (!selmon->showbar)
      wc.y = -bh;
    else if (selmon->showbar) {
      wc.y = 0;
      if (!selmon->topbar) wc.y = selmon->mh - bh;
    }
    XConfigureWindow(dpy, systray->win, CWY, &wc);
  }
  arrange(selmon);
}

void togglefullscr(const Arg* arg) {
  if (selmon->sel) setfullscreen(selmon->sel, !selmon->sel->isfullscreen);
}

void shiftview(const Arg *arg) {
  int ws = selmon->current_ws + arg->i;
  if (ws < 0) ws = WS_COUNT - 1;
  else if (ws >= WS_COUNT) ws = 0;
  view(&(Arg){.i = ws});
}

Monitor* get_neighbor_monitor(int dir) {
  if (dir > 0) {
    return selmon->next ? selmon->next : mons;
  } else {
    return selmon->prev ? selmon->prev : mons;
  }
}

void move_window_to_monitor(const Arg *arg) {
	if (!selmon->sel || !mons->next)
		return;
	sendmon(selmon->sel, get_neighbor_monitor(arg->i));
}

void updatecurrentdesktop(void) {
  long data[] = {selmon->current_ws};
  XChangeProperty(dpy, root, netatom[NetCurrentDesktop], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 1);
}
