PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
SYSCONFDIR ?= /etc

.PHONY: build install uninstall

build:
	dagger call build --source=. export --path=./bin/srwm

install:
	install -Dm755 target/release/driftwm $(DESTDIR)$(BINDIR)/driftwm
	install -Dm755 resources/driftwm-session $(DESTDIR)$(BINDIR)/driftwm-session
	install -Dm644 resources/driftwm.desktop $(DESTDIR)$(DATADIR)/wayland-sessions/driftwm.desktop
	install -Dm644 resources/driftwm-portals.conf $(DESTDIR)$(DATADIR)/xdg-desktop-portal/driftwm-portals.conf
	install -Dm644 config.example.toml $(DESTDIR)$(SYSCONFDIR)/driftwm/config.toml
	for f in extras/wallpapers/*.glsl; do \
		install -Dm644 "$$f" "$(DESTDIR)$(DATADIR)/driftwm/wallpapers/$$(basename $$f)"; \
	done

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/driftwm
	rm -f $(DESTDIR)$(BINDIR)/driftwm-session
	rm -f $(DESTDIR)$(DATADIR)/wayland-sessions/driftwm.desktop
	rm -f $(DESTDIR)$(DATADIR)/xdg-desktop-portal/driftwm-portals.conf
	rm -rf $(DESTDIR)$(DATADIR)/driftwm
	rm -rf $(DESTDIR)$(SYSCONFDIR)/driftwm
