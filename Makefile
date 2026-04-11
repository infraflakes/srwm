.PHONY: build build-all build-verbose build-all-verbose fmt install uninstall test clean

# Default OS if you just hit enter
DEFAULT_DISTRO := debian

build: clean
	@echo "Select target distro [debian|arch|fedora] (default: $(DEFAULT_DISTRO)):"
	@read -p "> " DISTRO; \
	SELECTED_DISTRO=$${DISTRO:-$(DEFAULT_DISTRO)}; \
	echo "Building for $$SELECTED_DISTRO..."; \
	dagger call build --source=. --distro=$$SELECTED_DISTRO export --path=./target/release/srwc-$$SELECTED_DISTRO

build-all: clean
	@echo "Launching parallel builds for distros"
	dagger call build-all --source=. export --path=./target/release

build-verbose: clean
	@echo "Select target distro [debian|arch|fedora] (default: $(DEFAULT_DISTRO)):"
	@read -p "> " DISTRO; \
	SELECTED_DISTRO=$${DISTRO:-$(DEFAULT_DISTRO)}; \
	echo "Building for $$SELECTED_DISTRO..."; \
	dagger call build --source=. --progress=plain --distro=$$SELECTED_DISTRO export --path=./target/release/srwc-$$SELECTED_DISTRO

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
