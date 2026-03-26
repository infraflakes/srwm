package core

/*
#include "bridge.h"
#include <stdlib.h>
*/
import "C"

type Scheme int

const (
	SchemeNorm       Scheme = 0
	SchemeSel        Scheme = 1
	SchemeTitle      Scheme = 2
	SchemeWorkspace  Scheme = 3
	SchemeWorkspace1 Scheme = 4
	SchemeWorkspace2 Scheme = 5
	SchemeWorkspace3 Scheme = 6
	SchemeWorkspace4 Scheme = 7
	SchemeWorkspace5 Scheme = 8
	SchemeWorkspace6 Scheme = 9
	SchemeWorkspace7 Scheme = 10
	SchemeWorkspace8 Scheme = 11
	SchemeWorkspace9 Scheme = 12
	TabSel           Scheme = 13
	TabNorm          Scheme = 14
	SchemeBtnPrev    Scheme = 15
	SchemeBtnNext    Scheme = 16
	SchemeBtnClose   Scheme = 17
)

func SetColor(scheme Scheme, slot int, hex string) {
	cs := C.CString(hex)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_color(C.int(scheme), C.int(slot), cs)
}

func SetWorkspace(idx int, name string) {
	cs := C.CString(name)
	// intentionally not freed — C core holds the pointer
	C.srwm_set_tag(C.int(idx), cs)
}

func SetWorkspacesLen(len int) {
	C.srwm_set_tags_len(C.int(len))
}

func SetWorkspaceColorfulOccupiedOnly(val bool) {
	ival := 0
	if val {
		ival = 1
	}
	C.srwm_set_tag_colorful_occupied_only(C.int(ival))
}

func SetWorkspaceScheme(idx int, scheme Scheme) {
	C.srwm_set_tagscheme(C.int(idx), C.int(scheme))
}
