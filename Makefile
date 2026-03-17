# Makefile for Serein Window Manager

VERSION ?= $(shell git describe --tags --always --dirty --first-parent 2>/dev/null || echo "dev")
.PHONY: all build config clean fmt lint

all: build

build:
	@echo "Building srwm $(VERSION)..."
	go build -ldflags="-w -s -X github.com/infraflakes/srwm/cmd.Version=$(VERSION)" -o ./bin/srwm .

fmt:
	@echo "Formatting code..."
	go fmt ./...

lint:
	@echo "Linting..."
	go vet ./...
	golangci-lint run

clean:
	@echo "Cleaning..."
	rm -rf ./bin/srwm
