package config

import (
	"github.com/infraflakes/srwm/internal/core"
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

	L.SetField(cfgTable, "px_till_snapping_to_screen_edge", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetPxTillSnappingToScreenEdge(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetPxTillSnappingToScreenEdge()))
		}
		return 1
	}))

	L.SetField(srwmMod, "cfg", cfgTable)

	// srwm.layout("monocle") or srwm.layout("canvas")
	L.SetField(srwmMod, "layout", L.NewFunction(func(L *lua.LState) int {
		name := L.CheckString(1)
		switch name {
		case "monocle":
			core.SetLayoutMode(0)
		case "canvas":
			core.SetLayoutMode(1)
		default:
			L.RaiseError("unknown layout: %s (expected 'monocle' or 'canvas')", name)
		}
		return 0
	}))
}
