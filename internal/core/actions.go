package core

/*
#include "bridge.h"
*/
import "C"

// ActionKillClient kills the currently focused client.
func ActionKillClient() {
	C.srwm_action_killclient()
}

// ActionToggleFullscreen toggles the fullscreen state of the currently focused client.
func ActionToggleFullscreen() {
	C.srwm_action_togglefullscr()
}

// ActionFocus focuses the next (+1) or previous (-1) client in the stack.
func ActionFocus(dir int) {
	C.srwm_action_focusstack(C.int(dir))
}

// ActionShiftWs shifts the view to the next (+1) or previous (-1) active workspace.
func ActionShiftWs(dir int) {
	C.srwm_action_shiftview(C.int(dir))
}

// ActionMoveToWs moves the currently focused client to the given workspace index (0-based).
func ActionMoveToWs(ws int) {
	C.srwm_action_move_to_ws(C.int(ws))
}

// ActionWsToNext moves the currently focused client to the next workspace.
func ActionWsToNext() {
	C.srwm_action_ws_to_next()
}

// ActionWsToPrev moves the currently focused client to the previous workspace.
func ActionWsToPrev() {
	C.srwm_action_ws_to_prev()
}

// ActionMoveWindowToMonitor moves the currently focused client to the next (+1) or previous (-1) monitor.
func ActionMoveWindowToMonitor(dir int) {
	C.srwm_action_move_window_to_monitor(C.int(dir))
}

// ActionShiftView shifts the view to the next (+1) or previous (-1) active tag.
func ActionShiftView(dir int) {
	C.srwm_action_shiftview(C.int(dir))
}

// ActionMoveToWs moves the currently focused client to the given workspace index (0-based).
func ActionMoveToWs(ws int) {
	C.srwm_action_move_to_ws(C.int(ws))
}

// ActionWsToNext moves the currently focused client to the next workspace.
func ActionWsToNext() {
	C.srwm_action_ws_to_next()
}

// ActionWsToPrev moves the currently focused client to the previous workspace.
func ActionWsToPrev() {
	C.srwm_action_ws_to_prev()
}

// ActionMoveWindowToMonitor moves the currently focused client to the next (+1) or previous (-1) monitor.
func ActionMoveWindowToMonitor(dir int) {
	C.srwm_action_move_window_to_monitor(C.int(dir))
}

// ActionView switches to the given workspace index (0-based).
func ActionView(ws int) {
	C.srwm_action_view(C.int(ws))
}

// ActionTag moves the currently focused client to the given workspace index (0-based).
func ActionTag(ws int) {
	C.srwm_action_tag(C.int(ws))
}

// ActionMoveCanvas pans the canvas: 0=left, 1=right, 2=up, 3=down.
func ActionMoveCanvas(dir int) {
	C.srwm_action_movecanvas(C.int(dir))
}

// ActionHomeCanvas resets the canvas viewport to origin.
func ActionHomeCanvas() {
	C.srwm_action_homecanvas()
}

// ActionCenterWindowOnCanvas centers the canvas on the focused window.
func ActionCenterWindowOnCanvas() {
	C.srwm_action_centerwindowoncanvas()
}

// ActionZoomCanvas zooms the canvas in (+1) or out (-1).
func ActionZoomCanvas(dir int) {
	C.srwm_action_zoomcanvas(C.int(dir))
}
