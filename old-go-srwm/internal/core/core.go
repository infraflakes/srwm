// Package core provides Go bindings to the srwm C window manager core.
//
// The C core handles all X11 state and event processing. This package
// exposes lifecycle control (Init, Run, Cleanup) and a status bar setter.
//
// Thread safety: Init() calls runtime.LockOSThread(). The core lifecycle
// functions (Run, Cleanup, Quit) MUST be called from the same goroutine
// that called Init(). However, SetStatus() and GrabKeys() are safe to
// call from other goroutines because the C core calls XInitThreads()
// during initialization.
package core

/*
#cgo CFLAGS: -std=c99 -pedantic -Wall -Wno-deprecated-declarations -O2
#cgo CFLAGS: -DXINERAMA -D_DEFAULT_SOURCE -D_BSD_SOURCE -D_XOPEN_SOURCE=700L
#cgo pkg-config: x11 xinerama xft fontconfig freetype2
#cgo LDFLAGS: -lXrender -lImlib2

#include "bridge.h"
#include <stdlib.h>
*/
import "C"

import (
	"errors"
	"runtime"
	"unsafe"
)

// InitDisplay opens the X display and checks for other WMs.
// Must be called before running Lua config.
//
// Locks the current goroutine to its OS thread — all subsequent core
// calls must happen on this same goroutine.
func InitDisplay() error {
	runtime.LockOSThread()
	if C.srwm_init_display() != 0 {
		runtime.UnlockOSThread()
		return errors.New("srwm: failed to initialize (cannot open display?)")
	}
	return nil
}

// InitSetup runs the X11 setup (reads fonts, colors, etc.) and scans
// existing windows. Call this AFTER Lua config has set values.
func InitSetup() {
	C.srwm_init_setup()
}

// Run enters the blocking X11 event loop. Returns when the WM is asked
// to quit or restart (running == 0).
func Run() {
	C.srwm_run()
}

// Cleanup tears down all X resources and closes the display.
func Cleanup() {
	C.srwm_cleanup()
}

// Quit signals the C event loop to stop (clean exit, not restart).
func Quit() {
	C.srwm_quit()
}

// ShouldRestart returns true if the event loop exited due to a restart
// request (Mod+Shift+R) rather than a quit.
func ShouldRestart() bool {
	return C.srwm_should_restart() != 0
}

// Restart signals the C event loop to stop and triggers a restart.
// The Go lifecycle loop in cmd/start.go will re-init the core and
// reload the Lua VM.
func Restart() {
	C.srwm_request_restart()
}

// SetStatus sets the X root window name, which srwm renders as the
// status bar text. Thread-safe (guarded by XInitThreads).
func SetStatus(text string) {
	cs := C.CString(text)
	defer C.free(unsafe.Pointer(cs))
	C.srwm_set_status(cs)
}

// StringToKeysym converts a human-readable X11 keysym name (e.g.
// "Return", "space", "XF86AudioMute") into its numeric KeySym value.
// Returns 0 (NoSymbol) if the name is not recognized.
//
// This is a thin wrapper around XStringToKeysym(3).
func StringToKeysym(name string) uint {
	cs := C.CString(name)
	defer C.free(unsafe.Pointer(cs))
	return uint(C.srwm_string_to_keysym(cs))
}

func b2i(b bool) int {
	if b {
		return 1
	}
	return 0
}
