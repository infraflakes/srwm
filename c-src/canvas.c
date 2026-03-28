#include "wm.h"

#define CANVAS_ZOOM_STEP 1.1f  
#define CANVAS_ZOOM_MIN  0.2f  
#define CANVAS_ZOOM_MAX  5.0f  

int compositor_running(void) {
    char atom_name[32];
    snprintf(atom_name, sizeof(atom_name), "_NET_WM_CM_S%d", DefaultScreen(dpy));
    Atom cm_atom = XInternAtom(dpy, atom_name, False);
    Window owner = XGetSelectionOwner(dpy, cm_atom);
    return (owner != None);
}

void publish_canvas_state(Monitor *m) {
    int32_t zoom_fp = (int32_t)(m->canvas_zoom * 10000.0f);
    XChangeProperty(dpy, root, netatom[SrwmCanvasZoom],
        XA_CARDINAL, 32, PropModeReplace, (unsigned char *)&zoom_fp, 1);
    
    int32_t cx = m->wx + m->ww / 2;
    int32_t cy = m->wy + m->wh / 2;
    XChangeProperty(dpy, root, netatom[SrwmCanvasCenterX],
        XA_CARDINAL, 32, PropModeReplace, (unsigned char *)&cx, 1);
    XChangeProperty(dpy, root, netatom[SrwmCanvasCenterY],
        XA_CARDINAL, 32, PropModeReplace, (unsigned char *)&cy, 1);
    
    int32_t active = 1;
    XChangeProperty(dpy, root, netatom[SrwmCanvasActive],
        XA_CARDINAL, 32, PropModeReplace, (unsigned char *)&active, 1);
    
    XFlush(dpy);
}

void movecanvas(const Arg *arg) {  
    int wsidx = selmon->current_ws;  
    int dx = 0, dy = 0;  
    int step = 120; /* MOVE_CANVAS_STEP */  
  
    switch(arg->i) {  
        case 0: dx = -step; break; /* left */  
        case 1: dx =  step; break; /* right */  
        case 2: dy = -step; break; /* up */  
        case 3: dy =  step; break; /* down */  
    }  
  
    selmon->canvas[wsidx].cx -= dx;  
    selmon->canvas[wsidx].cy -= dy;  
  
    Client *c;  
    for (c = selmon->clients; c; c = c->next) {  
        if (ISVISIBLE(c)) {  
            c->x -= dx;  
            c->y -= dy;  
            XMoveWindow(dpy, c->win, c->x, c->y);  
        }  
    }  
    drawtabs();  
}  
 
void homecanvas(const Arg *arg) {  
   int wsidx = selmon->current_ws;
    int cx = selmon->canvas[wsidx].cx;  
    int cy = selmon->canvas[wsidx].cy;  
   
    Client *c;  
    for (c = selmon->clients; c; c = c->next) {  
        if (ISVISIBLE(c)) {  
            c->x -= cx;  
            c->y -= cy;  
            XMoveWindow(dpy, c->win, c->x, c->y);  
        }  
    }  
   
    selmon->canvas[wsidx].cx = 0;  
    selmon->canvas[wsidx].cy = 0;  
    /* Reset zoom to 1.0 */  
    float old_zoom = selmon->canvas_zoom;  
    if (old_zoom != 1.0f) {  
        if (compositor_running()) {
            float scale = 1.0f / old_zoom;
            int scx = selmon->wx + selmon->ww / 2;
            int scy = selmon->wy + selmon->wh / 2;
            for (c = selmon->clients; c; c = c->next) {
                if (ISVISIBLE(c)) {
                    int new_x = scx + (int)((c->x - scx) * scale);
                    int new_y = scy + (int)((c->y - scy) * scale);
                    c->x = new_x;
                    c->y = new_y;
                    XMoveWindow(dpy, c->win, c->x, c->y);
                }
            }
            selmon->canvas_zoom = 1.0f;
            publish_canvas_state(selmon);
        } else {
            float scale = 1.0f / old_zoom;  
            int scx = selmon->wx + selmon->ww / 2;  
            int scy = selmon->wy + selmon->wh / 2;  
            for (c = selmon->clients; c; c = c->next) {  
                if (ISVISIBLE(c)) {  
                    int new_x = scx + (int)((c->x - scx) * scale);  
                    int new_y = scy + (int)((c->y - scy) * scale);  
                    int new_w = MAX(1, (int)(c->w * scale));  
                    int new_h = MAX(1, (int)(c->h * scale));  
                    resizeclient(c, new_x, new_y, new_w, new_h);  
                }  
            }  
            selmon->canvas_zoom = 1.0f;  
        }
    }
    drawtabs();  
    XFlush(dpy);  
}
  
void centerwindowoncanvas(const Arg *arg) {  
    Client *c = selmon->sel;  
    if (!c)  
        return;  
  
    Monitor *m = c->mon;  
    int wsidx = m->current_ws;  
  
    int screen_center_x = m->wx + (m->ww / 2);  
    int screen_center_y = m->wy + (m->wh / 2);  
    int win_center_x = c->x + (c->w + 2 * c->bw) / 2;  
    int win_center_y = c->y + (c->h + 2 * c->bw) / 2;  
  
    int dx = screen_center_x - win_center_x;  
    int dy = screen_center_y - win_center_y;  
  
    if (dx == 0 && dy == 0)  
        return;  
  
    Client *tmp;  
    for (tmp = m->clients; tmp; tmp = tmp->next) {  
        if (ISVISIBLE(tmp)) {  
            tmp->x += dx;  
            tmp->y += dy;  
            XMoveWindow(dpy, tmp->win, tmp->x, tmp->y);  
        }  
    }  
  
    m->canvas[wsidx].cx += dx;  
    m->canvas[wsidx].cy += dy;  
    drawtabs();  
}  
 
void manuallymovecanvas(const Arg *arg) {  
    int start_x, start_y;
    Window dummy;  
    int di;  
    unsigned int dui;  
    int wsidx = selmon->current_ws;  
    Time lasttime = 0;  
  
    if (selmon->sel && selmon->sel->isfullscreen)  
        return;  
    if (!XQueryPointer(dpy, root, &dummy, &dummy, &start_x, &start_y, &di, &di, &dui))  
        return;  
    if (XGrabPointer(dpy, root, False, MOUSEMASK, GrabModeAsync, GrabModeAsync,  
        None, cursor[CurMove]->cursor, CurrentTime) != GrabSuccess)  
        return;  
  
    XEvent ev;  
    do {  
        XMaskEvent(dpy, MOUSEMASK|ExposureMask|SubstructureRedirectMask, &ev);  
        switch (ev.type) {  
        case MotionNotify:  
        {  
            if ((ev.xmotion.time - lasttime) <= (1000 / 60))  
                continue;  
            lasttime = ev.xmotion.time;  
  
            float zoom = selmon->canvas_zoom;
            float speed = 3.0f;  // base multiplier
            int nx = (int)((ev.xmotion.x - start_x) * speed / zoom);
            int ny = (int)((ev.xmotion.y - start_y) * speed / zoom);
  
            Client *c;  
            for (c = selmon->clients; c; c = c->next) {  
                if (ISVISIBLE(c)) {  
                    c->x += nx;  
                    c->y += ny;  
                    XMoveWindow(dpy, c->win, c->x, c->y);  
                }  
            }  
  
            selmon->canvas[wsidx].cx += nx;
            selmon->canvas[wsidx].cy += ny;
            drawtabs();
            start_x = ev.xmotion.x;
            start_y = ev.xmotion.y;
        }   break;
        }
    } while (ev.type != ButtonRelease);

    XUngrabPointer(dpy, CurrentTime);
}  
 
void zoomcanvas(const Arg *arg) {  
   int wsidx = selmon->current_ws;
    float old_zoom = selmon->canvas_zoom;  
    float new_zoom;  
   
    if (arg->i > 0)  
        new_zoom = old_zoom * CANVAS_ZOOM_STEP;  /* zoom in */  
    else  
        new_zoom = old_zoom / CANVAS_ZOOM_STEP;  /* zoom out */  
   
    /* clamp */  
    if (new_zoom < CANVAS_ZOOM_MIN) new_zoom = CANVAS_ZOOM_MIN;  
    if (new_zoom > CANVAS_ZOOM_MAX) new_zoom = CANVAS_ZOOM_MAX;  
    if (new_zoom == old_zoom)  
        return;  
   
    if (compositor_running()) {
        selmon->canvas_zoom = new_zoom;
        publish_canvas_state(selmon);
    } else {
        float scale = new_zoom / old_zoom;  
        int cx = selmon->wx + selmon->ww / 2;  
        int cy = selmon->wy + selmon->wh / 2;  
   
        Client *c;  
        for (c = selmon->clients; c; c = c->next) {  
            if (ISVISIBLE(c)) {  
                int new_x = cx + (int)((c->x - cx) * scale);  
                int new_y = cy + (int)((c->y - cy) * scale);  
                int new_w = MAX(1, (int)(c->w * scale));  
                int new_h = MAX(1, (int)(c->h * scale));  
   
                resizeclient(c, new_x, new_y, new_w, new_h);  
            }  
        }  
   
        selmon->canvas[wsidx].cx = (int)(selmon->canvas[wsidx].cx * scale);  
        selmon->canvas[wsidx].cy = (int)(selmon->canvas[wsidx].cy * scale);  
        selmon->canvas_zoom = new_zoom;  
    }
   
    drawtabs();  
}

// Returns 1 if panning occurred, 0 otherwise.  
// `exclude` is an optional client to skip when moving windows (e.g., the window being dragged).  
// `pan_dx_out` and `pan_dy_out` return the pan delta applied (for callers that need to adjust drag references).  
int canvas_edge_autopan(int cursor_x, int cursor_y, Client *exclude, int *pan_dx_out, int *pan_dy_out) {  
    int wsidx = selmon->current_ws;  
    float zoom = selmon->canvas_zoom;  
    if (zoom >= 1.0f)  
        return 0;  
      
    int edge_margin = 6;   // pixels from edge to trigger pan  
    float base_pan_speed = 22.5; // base pixels per frame  
    // Scale pan speed inversely with zoom so it feels consistent  
    int pan_speed = (int)(base_pan_speed / zoom);  
      
    int pan_dx = 0, pan_dy = 0;  
      
    if (cursor_x <= selmon->mx + edge_margin)  
        pan_dx = -pan_speed;  
    else if (cursor_x >= selmon->mx + selmon->mw - edge_margin - 1)  
        pan_dx = pan_speed;  
    if (cursor_y <= selmon->my + edge_margin)  
        pan_dy = -pan_speed;  
    else if (cursor_y >= selmon->my + selmon->mh - edge_margin - 1)  
        pan_dy = pan_speed;  
      
    if (!pan_dx && !pan_dy)  
        return 0;  
      
    // Move all visible clients except the excluded one  
    Client *c;  
    for (c = selmon->clients; c; c = c->next) {  
        if (ISVISIBLE(c) && c != exclude) {  
            c->x -= pan_dx;  
            c->y -= pan_dy;  
            XMoveWindow(dpy, c->win, c->x, c->y);  
        }  
    }  
      
    // Update canvas offset  
    selmon->canvas[wsidx].cx -= pan_dx;  
    selmon->canvas[wsidx].cy -= pan_dy;  
      
    if (pan_dx_out) *pan_dx_out = pan_dx;  
    if (pan_dy_out) *pan_dy_out = pan_dy;  
      
    return 1;  
}
