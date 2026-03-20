package core

/*
#include "bridge.h"
#include <stdlib.h>
*/
import "C"

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
