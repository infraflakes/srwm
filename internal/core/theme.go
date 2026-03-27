package core

/*
#include "bridge.h"
#include <stdlib.h>
*/
import "C"

type Scheme int

const (
	TabSel         Scheme = 0
	TabNorm        Scheme = 1
	SchemeBtnPrev  Scheme = 2
	SchemeBtnNext  Scheme = 3
	SchemeBtnClose Scheme = 4
)

func SetColor(scheme Scheme, slot int, hex string) {
	cs := C.CString(hex)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_color(C.int(scheme), C.int(slot), cs)
}

func SetBorderActive(hex string) {
	cs := C.CString(hex)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_border_active(cs)
}

func SetBorderInactive(hex string) {
	cs := C.CString(hex)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_border_inactive(cs)
}

func SetWorkspace(idx int, name string) {
	cs := C.CString(name)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_ws_label(C.int(idx), cs)
}

func SetWorkspacesLen(len int) {
	C.srwm_set_ws_count(C.int(len))
}

func SetWorkspaceScheme(idx int, scheme Scheme) {
	C.srwm_set_ws_scheme(C.int(idx), C.int(scheme))
}
