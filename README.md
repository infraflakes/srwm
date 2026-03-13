# Serein Window Manager
A fully statically linked dynamic X11 window manager with CGO

# How to build

Current the project only supports flakes and direnv.
After having all the dependencies you can easily build the window manager:

```bash
make build
```

# Starting the WM

You can generate the default config with `srwm kickstart`:

```bash
srwm kickstart
```

Then start the window manager with `srwm start` and any X11 graphical sessions launcher, for example with `sx`:
```bash
sx srwm start
```

---
