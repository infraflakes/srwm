void
warptoclient(Client *c)
{
    int x, y;
    
    if (!c) {
        XWarpPointer(dpy, None, root, 0, 0, 0, 0, selmon->wx + selmon->ww/2, selmon->wy + selmon->wh/2);
        return;
    }
    
    x = c->x + c->w / 2;
    y = c->y + c->h / 2;
    
    XWarpPointer(dpy, None, root, 0, 0, 0, 0, x, y);
}
