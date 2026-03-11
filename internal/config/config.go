package config

import (
	"github.com/nixuris/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

func RegisterConfigAPI(L *lua.LState, srwmMod *lua.LTable) {
	cfgTable := L.NewTable()

	L.SetField(cfgTable, "borderpx", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetBorderPx(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetBorderPx()))
		}
		return 1
	}))

	L.SetField(cfgTable, "gaps", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetGaps(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetGaps()))
		}
		return 1
	}))

	L.SetField(cfgTable, "showbar", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetShowbar(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetShowbar()))
		}
		return 1
	}))

	L.SetField(cfgTable, "topbar", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTopBar(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTopBar()))
		}
		return 1
	}))

	L.SetField(cfgTable, "toptab", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTopTab(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTopTab()))
		}
		return 1
	}))

	L.SetField(cfgTable, "systray_enable", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetSystrayEnable(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetSystrayEnable()))
		}
		return 1
	}))

	L.SetField(cfgTable, "systray_spacing", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetSystraySpacing(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetSystraySpacing()))
		}
		return 1
	}))

	L.SetField(cfgTable, "systray_pinning", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetSystrayPinning(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetSystrayPinning()))
		}
		return 1
	}))

	L.SetField(cfgTable, "bar_horizontal_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetBarHorizontalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetBarHorizontalPadding()))
		}
		return 1
	}))

	L.SetField(cfgTable, "bar_vertical_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetBarVerticalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetBarVerticalPadding()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tab_vertical_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabVerticalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabVerticalPadding()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tab_in_horizontal_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabInHorizontalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabInHorizontalPadding()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tab_out_horizontal_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabOutHorizontalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabOutHorizontalPadding()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tag_preview_size", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagPreviewSize(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTagPreviewSize()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tag_preview_enable", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagPreviewEnable(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTagPreviewEnable()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tag_underline_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlinePadding(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetTagUnderlinePadding()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tag_underline_size", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlineSize(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetTagUnderlineSize()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tag_underline_offset_from_bar_bottom", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlineOffsetFromBarBottom(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetTagUnderlineOffsetFromBarBottom()))
		}
		return 1
	}))

	L.SetField(cfgTable, "tag_underline_for_all_tags", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlineForAllTags(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTagUnderlineForAllTags()))
		}
		return 1
	}))

	L.SetField(cfgTable, "px_till_snapping_to_screen_edge", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetPxTillSnappingToScreenEdge(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetPxTillSnappingToScreenEdge()))
		}
		return 1
	}))

	L.SetField(srwmMod, "cfg", cfgTable)
}
