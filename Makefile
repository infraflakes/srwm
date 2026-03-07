# Makefile for Serein Window Manager

VERSION ?= $(shell git describe --tags --always --dirty --first-parent 2>/dev/null || echo "dev")

.PHONY: all build generate fmt lint clean

all: build
generate:
	@echo "Building swm..."
	go build -ldflags="-w -X main.Version=$(VERSION)" -o ./bin/swm

fmt:
	@echo "Formatting code..."
	go fmt ./...

lint:
	CGO_ENABLED=1 CC="zig cc -target x86_64-windows-gnu" GOOS=windows golangci-lint run

clean:
	@echo "Cleaning..."
	rm -rf ./bin/swm
