package core

/*
#include "bridge.h"
#include <stdlib.h>
*/
import "C"

func GetLayoutMode() int {
	return int(C.srwm_get_layout_mode())
}

func SetLayoutMode(v int) {
	C.srwm_set_layout_mode(C.int(v))
}

func GetBorderPx() uint {
	return uint(C.srwm_get_borderpx())
}

func SetBorderPx(v uint) {
	C.srwm_set_borderpx(C.uint(v))
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
