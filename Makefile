PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
SYSCONFDIR ?= /etc

.PHONY: build build-verbose fmt install uninstall

build:
	dagger call build --source=. export --path=./target/release/srwm

build-verbose:
	dagger call build --source=. --progress=plain export --path=./target/release/srwm

fmt:
	cargo fmt

install:
	install -Dm755 target/release/srwm $(DESTDIR)$(BINDIR)/srwm
	install -Dm755 resources/srwm-session $(DESTDIR)$(BINDIR)/srwm-session
	install -Dm644 resources/srwm.desktop $(DESTDIR)$(DATADIR)/wayland-sessions/srwm.desktop
	install -Dm644 resources/srwm-portals.conf $(DESTDIR)$(DATADIR)/xdg-desktop-portal/srwm-portals.conf
	install -Dm644 config.example.toml $(DESTDIR)$(SYSCONFDIR)/srwm/config.toml
	for f in extras/wallpapers/*.glsl; do \
		install -Dm644 "$$f" "$(DESTDIR)$(DATADIR)/srwm/wallpapers/$$(basename $$f)"; \
	done

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/srwm
	rm -f $(DESTDIR)$(BINDIR)/srwm-session
	rm -f $(DESTDIR)$(DATADIR)/wayland-sessions/srwm.desktop
	rm -f $(DESTDIR)$(DATADIR)/xdg-desktop-portal/srwm-portals.conf
	rm -rf $(DESTDIR)$(DATADIR)/srwm
	rm -rf $(DESTDIR)$(SYSCONFDIR)/srwm
