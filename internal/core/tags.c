#include "wm.h"

void view(const Arg* arg) {
  int i;
  unsigned int tmptag;

  if ((arg->ui & TAGMASK) == selmon->tagset[selmon->seltags]) return;
  selmon->seltags ^= 1;
  if (arg->ui & TAGMASK) {
    selmon->prevtag = selmon->curtag;
    selmon->tagset[selmon->seltags] = arg->ui & TAGMASK;

    if (arg->ui == ~0)
      selmon->curtag = 0;
    else {
      for (i = 0; !(arg->ui & 1 << i); i++);
      selmon->curtag = i + 1;
    }
  } else {
    tmptag = selmon->prevtag;
    selmon->prevtag = selmon->curtag;
    selmon->curtag = tmptag;
  }

  if (selmon->showbar != (int)((selmon->showbar_mask >> selmon->curtag) & 1))
    togglebar(NULL);
  focus(NULL);
  arrange(selmon);
  updatecurrentdesktop();
}

void toggleview(const Arg* arg) {
  unsigned int newtagset = selmon->tagset[selmon->seltags] ^ (arg->ui & TAGMASK);
  int i;

  if (newtagset) {
    selmon->tagset[selmon->seltags] = newtagset;

    if (newtagset == ~0) {
      selmon->prevtag = selmon->curtag;
      selmon->curtag = 0;
    }

    if (!(newtagset & 1 << (selmon->curtag - 1))) {
      selmon->prevtag = selmon->curtag;
      for (i = 0; !(newtagset & 1 << i); i++);
      selmon->curtag = i + 1;
    }

    if (selmon->showbar != (int)((selmon->showbar_mask >> selmon->curtag) & 1)) togglebar(NULL);

    focus(NULL);
    arrange(selmon);
  }
  updatecurrentdesktop();
}

void tag(const Arg* arg) {
  Client* c;
  if (selmon->sel && arg->ui & TAGMASK) {
    c = selmon->sel;
    selmon->sel->tags = arg->ui & TAGMASK;
    selmon->occ = 0;
    for (Client* t = selmon->clients; t; t = t->next)
      selmon->occ |= t->tags;
    setclienttagprop(c);
    focus(NULL);
    arrange(selmon);
  }
}

void togglebar(const Arg* arg) {
  selmon->showbar = !selmon->showbar;
  if (selmon->showbar)
    selmon->showbar_mask |=  (1u << selmon->curtag);
  else
    selmon->showbar_mask &= ~(1u << selmon->curtag);
  updatebarpos(selmon);
  resizebarwin(selmon);
  if (systray_enable) {
    XWindowChanges wc;
    if (!selmon->showbar)
      wc.y = -bh;
    else if (selmon->showbar) {
      wc.y = selmon->gap;
      if (!selmon->topbar) wc.y = selmon->mh - bh + selmon->gap;
    }
    XConfigureWindow(dpy, systray->win, CWY, &wc);
  }
  arrange(selmon);
}

void toggletag(const Arg* arg) {
  unsigned int newtags;

  if (!selmon->sel) return;
  newtags = selmon->sel->tags ^ (arg->ui & TAGMASK);
  if (newtags) {
    selmon->sel->tags = newtags;
    selmon->occ = 0;
    for (Client* t = selmon->clients; t; t = t->next)
      selmon->occ |= t->tags;
    setclienttagprop(selmon->sel);
    focus(NULL);
    arrange(selmon);
  }
  updatecurrentdesktop();
}

void togglefloating(const Arg* arg) {
  if (!selmon->sel) return;
  if (selmon->sel->isfullscreen) return;
  if (selmon->canvas_mode) return;  /* all windows are floating in canvas mode */
  selmon->sel->isfloating = !selmon->sel->isfloating || selmon->sel->isfixed;
  if (selmon->sel->isfloating)
    resize(selmon->sel, selmon->sel->x, selmon->sel->y, selmon->sel->w, selmon->sel->h, 0);
  arrange(selmon);
}

void togglefullscr(const Arg* arg) {
  if (selmon->sel) setfullscreen(selmon->sel, !selmon->sel->isfullscreen);
}

void shiftview(const Arg *arg) {
  Arg shifted;
 
  if(arg->i > 0) // left circular shift
    shifted.ui = (selmon->tagset[selmon->seltags] << arg->i)
      | (selmon->tagset[selmon->seltags] >> (TAGSLENGTH - arg->i));
 
  else // right circular shift
    shifted.ui = selmon->tagset[selmon->seltags] >> (- arg->i)
      | selmon->tagset[selmon->seltags] << (TAGSLENGTH + arg->i);
  shifted.ui &= TAGMASK; //ensures bits beyond tags_len are zeroed out
  view(&shifted);
}

Monitor* get_neighbor_monitor(int dir) {
  if (dir > 0) {
    return selmon->next ? selmon->next : mons;
  } else {
    return selmon->prev ? selmon->prev : mons;
  }
}

void move_tag_to_monitor(const Arg *arg) {
	if (!selmon->sel || !mons->next)
		return;
	sendmon(selmon->sel, get_neighbor_monitor(arg->i));
}

void updatecurrentdesktop(void) {
  long rawdata[] = {selmon->tagset[selmon->seltags]};
  int i = 0;
  while (*rawdata >> (i + 1)) {
    i++;
  }
  long data[] = {i};
  XChangeProperty(dpy, root, netatom[NetCurrentDesktop], XA_CARDINAL, 32,
                  PropModeReplace, (unsigned char*)data, 1);
}
