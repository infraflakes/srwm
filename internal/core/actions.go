package core

/*
#include "bridge.h"
*/
import "C"

func ActionKillClient() {
	C.srwm_action_killclient()
}

func ActionToggleFullscreen() {
	C.srwm_action_togglefullscr()
}

func ActionFocus(dir int) {
	C.srwm_action_focusstack(C.int(dir))
}

func ActionShiftWs(dir int) {
	C.srwm_action_shiftview(C.int(dir))
}

func ActionMoveToWs(ws int) {
	C.srwm_action_move_to_ws(C.int(ws))
}

func ActionWsToNext() {
	C.srwm_action_ws_to_next()
}

func ActionWsToPrev() {
	C.srwm_action_ws_to_prev()
}

func ActionMoveWindowToMonitor(dir int) {
	C.srwm_action_move_window_to_monitor(C.int(dir))
}

func ActionView(ws int) {
	C.srwm_action_view(C.int(ws))
}

func ActionMoveCanvas(dir int) {
	C.srwm_action_movecanvas(C.int(dir))
}

func ActionHomeCanvas() {
	C.srwm_action_homecanvas()
}

func ActionCenterWindowOnCanvas() {
	C.srwm_action_centerwindowoncanvas()
}

func ActionZoomCanvas(dir int) {
	C.srwm_action_zoomcanvas(C.int(dir))
}
