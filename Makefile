# Makefile for Serein Window Manager

VERSION ?= $(shell git describe --tags --always --dirty --first-parent 2>/dev/null || echo "dev")
.PHONY: all build config clean fmt lint

all: build

build: clean
	@echo "Building srwm $(VERSION)..."
	@mkdir -p bin
	dagger call build --source=. export --path=./target/release/srwm

fmt:
	@echo "Formatting code..."
	cargo fmt

lint:
	@echo "Linting..."

clean:
	@echo "Cleaning..."
	rm -rf ./target/release
