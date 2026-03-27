package core

/*
#include "bridge.h"
#include <stdlib.h>
*/
import "C"

func GetBorderPx() uint {
	return uint(C.srwm_get_borderpx())
}

func SetBorderPx(v uint) {
	C.srwm_set_borderpx(C.uint(v))
}

func GetTabHeight() int {
	return int(C.srwm_get_tab_height())
}

func SetTabHeight(v int) {
	C.srwm_set_tab_height(C.int(v))
}

func GetTabTileVerticalPadding() int {
	return int(C.srwm_get_tab_tile_vertical_padding())
}

func SetTabTileVerticalPadding(v int) {
	C.srwm_set_tab_tile_vertical_padding(C.int(v))
}

func GetTabTileOuterPaddingHorizontal() int {
	return int(C.srwm_get_tab_tile_outer_padding_horizontal())
}

func SetTabTileOuterPaddingHorizontal(v int) {
	C.srwm_set_tab_tile_outer_padding_horizontal(C.int(v))
}

func GetTabTileInnerPaddingHorizontal() int {
	return int(C.srwm_get_tab_tile_inner_padding_horizontal())
}

func SetTabTileInnerPaddingHorizontal(v int) {
	C.srwm_set_tab_tile_inner_padding_horizontal(C.int(v))
}

func GetTopTab() bool {
	return C.srwm_get_toptab() != 0
}

func SetTopTab(v bool) {
	C.srwm_set_toptab(C.int(b2i(v)))
}

func SetFont(font string) {
	cs := C.CString(font)
	// Note: intentionally not freed — C core holds this pointer for the WM lifetime
	C.srwm_set_font(cs)
}
