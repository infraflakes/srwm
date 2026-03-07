# Makefile for Serein Window Manager

VERSION ?= $(shell git describe --tags --always --dirty --first-parent 2>/dev/null || echo "dev")
.PHONY: all build config clean fmt lint

all: build

build:
	@echo "Building swm $(VERSION)..."
	go build -ldflags="-w -s -X main.Version=$(VERSION)" -o ./bin/swm .
	upx --best --lzma ./bin/swm

fmt:
	@echo "Formatting code..."
	go fmt ./...

lint:
	@echo "Linting..."
	golangci-lint run

clean:
	@echo "Cleaning..."
	rm -rf ./bin/swm
	rm -f $(CONFIG_H)
