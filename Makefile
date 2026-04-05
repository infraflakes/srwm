PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
SYSCONFDIR ?= /etc

.PHONY: build build-all build-verbose build-all-verbose fmt install uninstall test clean

# Default OS if you just hit enter
DEFAULT_OS=debian

build: clean
	@echo "Select target OS [debian|arch|fedora] (default: $(DEFAULT_OS)):"
	@read -p "> " OS; \
	SELECTED_OS=$${OS:-$(DEFAULT_OS)}; \
	echo "Building for $$SELECTED_OS..."; \
	dagger call build --source=. --os=$$SELECTED_OS export --path=./target/release/srwm-$$SELECTED_OS

build-all: clean
	@echo "Launching parallel builds for distros"
	dagger call build-all --source=. export --path=./target/release

build-verbose: clean
	@echo "Select target OS [debian|arch|fedora] (default: $(DEFAULT_OS)):"
	@read -p "> " OS; \
	SELECTED_OS=$${OS:-$(DEFAULT_OS)}; \
	echo "Building for $$SELECTED_OS..."; \
	dagger call build --source=. --os=$$SELECTED_OS --progress=plain export --path=./target/release/srwm-$$SELECTED_OS

build-all-verbose: clean
	@echo "Launching parallel builds for distros"
	dagger call build-all --source=. --progress=plain export --path=./target/release

test:
	@echo "Running tests..."
	dagger call test --source=. --progress=plain

fmt:
	cargo fmt

clean:
	rm -rf target
