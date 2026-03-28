# Makefile for Serein Window Manager

VERSION ?= $(shell git describe --tags --always --dirty --first-parent 2>/dev/null || echo "dev")
.PHONY: all build config clean fmt lint

all: build

build:
	@echo "Building srwm $(VERSION)..."
	dagger call build --source=.

fmt:
	@echo "Formatting code..."
	cargo fmt

lint:
	@echo "Linting..."

clean:
	@echo "Cleaning..."
	rm -rf ./target
	rm -rf ./bin
