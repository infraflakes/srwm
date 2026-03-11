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

// SetStatus sets the X root window name, which dwm renders as the
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

func GetBorderPx() uint {
	return uint(C.srwm_get_borderpx())
}

func SetBorderPx(v uint) {
	C.srwm_set_borderpx(C.uint(v))
}

func GetPxTillSnappingToScreenEdge() uint {
	return uint(C.srwm_get_px_till_snapping_to_screen_edge())
}

func SetPxTillSnappingToScreenEdge(v uint) {
	C.srwm_set_px_till_snapping_to_screen_edge(C.uint(v))
}

func GetGaps() uint {
	return uint(C.srwm_get_gaps())
}

func SetGaps(v uint) {
	C.srwm_set_gaps(C.uint(v))
}

func GetSystrayPinning() uint {
	return uint(C.srwm_get_systraypinning())
}

func SetSystrayPinning(v uint) {
	C.srwm_set_systraypinning(C.uint(v))
}

func GetSystraySpacing() uint {
	return uint(C.srwm_get_systrayspacing())
}

func SetSystraySpacing(v uint) {
	C.srwm_set_systrayspacing(C.uint(v))
}

func GetSystrayEnable() bool {
	return C.srwm_get_systray_enable() != 0
}

func SetSystrayEnable(v bool) {
	C.srwm_set_systray_enable(C.int(b2i(v)))
}

func GetShowbar() bool {
	return C.srwm_get_showbar() != 0
}

func SetShowbar(v bool) {
	C.srwm_set_showbar(C.int(b2i(v)))
}

func GetBarHorizontalPadding() int {
	return int(C.srwm_get_bar_horizontal_padding())
}

func SetBarHorizontalPadding(v int) {
	C.srwm_set_bar_horizontal_padding(C.int(v))
}

func GetBarVerticalPadding() int {
	return int(C.srwm_get_bar_vertical_padding())
}

func SetBarVerticalPadding(v int) {
	C.srwm_set_bar_vertical_padding(C.int(v))
}

func GetTabVerticalPadding() int {
	return int(C.srwm_get_tab_vertical_padding())
}

func SetTabVerticalPadding(v int) {
	C.srwm_set_tab_vertical_padding(C.int(v))
}

func GetTabInHorizontalPadding() int {
	return int(C.srwm_get_tab_in_horizontal_padding())
}

func SetTabInHorizontalPadding(v int) {
	C.srwm_set_tab_in_horizontal_padding(C.int(v))
}

func GetTabOutHorizontalPadding() int {
	return int(C.srwm_get_tab_out_horizontal_padding())
}

func SetTabOutHorizontalPadding(v int) {
	C.srwm_set_tab_out_horizontal_padding(C.int(v))
}

func GetTagPreviewSize() int {
	return int(C.srwm_get_tag_preview_size())
}

func SetTagPreviewSize(v int) {
	C.srwm_set_tag_preview_size(C.int(v))
}

func GetTagPreviewEnable() bool {
	return C.srwm_get_tag_preview_enable() != 0
}

func SetTagPreviewEnable(v bool) {
	C.srwm_set_tag_preview_enable(C.int(b2i(v)))
}

func GetTagUnderlinePadding() uint {
	return uint(C.srwm_get_tag_underline_padding())
}

func SetTagUnderlinePadding(v uint) {
	C.srwm_set_tag_underline_padding(C.uint(v))
}

func GetTagUnderlineSize() uint {
	return uint(C.srwm_get_tag_underline_size())
}

func SetTagUnderlineSize(v uint) {
	C.srwm_set_tag_underline_size(C.uint(v))
}

func GetTagUnderlineOffsetFromBarBottom() uint {
	return uint(C.srwm_get_tag_underline_offset_from_bar_bottom())
}

func SetTagUnderlineOffsetFromBarBottom(v uint) {
	C.srwm_set_tag_underline_offset_from_bar_bottom(C.uint(v))
}

func GetTagUnderlineForAllTags() bool {
	return C.srwm_get_tag_underline_for_all_tags() != 0
}

func SetTagUnderlineForAllTags(v bool) {
	C.srwm_set_tag_underline_for_all_tags(C.int(b2i(v)))
}

func GetTopTab() bool {
	return C.srwm_get_toptab() != 0
}

func SetTopTab(v bool) {
	C.srwm_set_toptab(C.int(b2i(v)))
}

func GetTopBar() bool {
	return C.srwm_get_topbar() != 0
}

func SetTopBar(v bool) {
	C.srwm_set_topbar(C.int(b2i(v)))
}

func SetFont(font string) {
	cs := C.CString(font)
	// Note: intentionally not freed — C core holds this pointer for the WM lifetime
	C.srwm_set_font(cs)
}

// Scheme mirrors the C enum in dwm.c — order must match exactly.
type Scheme int

const (
	SchemeNorm     Scheme = 0
	SchemeSel      Scheme = 1
	SchemeTitle    Scheme = 2
	SchemeTag      Scheme = 3
	SchemeTag1     Scheme = 4
	SchemeTag2     Scheme = 5
	SchemeTag3     Scheme = 6
	SchemeTag4     Scheme = 7
	SchemeTag5     Scheme = 8
	SchemeTag6     Scheme = 9
	SchemeTag7     Scheme = 10
	SchemeTag8     Scheme = 11
	SchemeTag9     Scheme = 12
	TabSel         Scheme = 13
	TabNorm        Scheme = 14
	SchemeBtnPrev  Scheme = 15
	SchemeBtnNext  Scheme = 16
	SchemeBtnClose Scheme = 17
)

func SetColor(scheme Scheme, slot int, hex string) {
	cs := C.CString(hex)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_color(C.int(scheme), C.int(slot), cs)
}

func SetTag(idx int, name string) {
	cs := C.CString(name)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_tag(C.int(idx), cs)
}

func SetTagsLen(len int) {
	C.srwm_set_tags_len(C.int(len))
}

func SetTagColorfulOccupiedOnly(val bool) {
	ival := 0
	if val {
		ival = 1
	}
	C.srwm_set_tag_colorful_occupied_only(C.int(ival))
}

func SetTagScheme(idx int, scheme Scheme) {
	C.srwm_set_tagscheme(C.int(idx), C.int(scheme))
}

func b2i(b bool) int {
	if b {
		return 1
	}
	return 0
}
