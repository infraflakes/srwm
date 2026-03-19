package core

/*
#include "bridge.h"
*/
import "C"

// ActionKillClient kills the currently focused client.
func ActionKillClient() {
	C.srwm_action_killclient()
}

// ActionToggleFloating toggles the floating state of the currently focused client.
func ActionToggleFloating() {
	C.srwm_action_togglefloating()
}

// ActionToggleFullscreen toggles the fullscreen state of the currently focused client.
func ActionToggleFullscreen() {
	C.srwm_action_togglefullscr()
}

// ActionFocus focuses the next (+1) or previous (-1) client in the stack.
func ActionFocus(dir int) {
	C.srwm_action_focusstack(C.int(dir))
}

// ActionShiftView shifts the view to the next (+1) or previous (-1) active tag.
func ActionShiftView(dir int) {
	C.srwm_action_shiftview(C.int(dir))
}

// ActionTagToPrev moves the currently focused client to the previous tag.
func ActionTagToPrev() {
	C.srwm_action_tagtoprev()
}

// ActionTagToNext moves the currently focused client to the next tag.
func ActionTagToNext() {
	C.srwm_action_tagtonext()
}

// ActionMoveTagToMonitor moves the currently focused client to the next (+1) or previous (-1) monitor.
func ActionMoveTagToMonitor(dir int) {
	C.srwm_action_move_tag_to_monitor(C.int(dir))
}

// ActionView views the tags matching the given bitmask.
func ActionView(mask uint) {
	C.srwm_action_view(C.uint(mask))
}

// ActionTag tags the currently focused client with the given bitmask.
func ActionTag(mask uint) {
	C.srwm_action_tag(C.uint(mask))
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
