package config

import (
	"github.com/nixuris/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

// RegisterActionsAPI sets up the srwm.window and srwm.tag namespaces.
func RegisterActionsAPI(L *lua.LState, srwmMod *lua.LTable) {
	// srwm.window namespace
	windowTable := L.NewTable()

	L.SetField(windowTable, "kill", L.NewFunction(func(L *lua.LState) int {
		core.ActionKillClient()
		return 0
	}))


	L.SetField(windowTable, "toggle_floating", L.NewFunction(func(L *lua.LState) int {
		core.ActionToggleFloating()
		return 0
	}))

	L.SetField(windowTable, "toggle_fullscreen", L.NewFunction(func(L *lua.LState) int {
		core.ActionToggleFullscreen()
		return 0
	}))

	L.SetField(windowTable, "focus", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionFocus(dir)
		return 0
	}))

	L.SetField(srwmMod, "window", windowTable)

	// srwm.tag namespace
	tagTable := L.NewTable()

	L.SetField(tagTable, "view_prev", L.NewFunction(func(L *lua.LState) int {
		core.ActionTagToPrev()
		return 0
	}))

	L.SetField(tagTable, "view_next", L.NewFunction(func(L *lua.LState) int {
		core.ActionTagToNext()
		return 0
	}))

	L.SetField(tagTable, "shift_view", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionShiftView(dir)
		return 0
	}))

	L.SetField(tagTable, "move_to_monitor", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionMoveTagToMonitor(dir)
		return 0
	}))

	// For mask functions, Lua passes a 1-based index (1-9) which we convert to a bitmask.
	indexToMask := func(L *lua.LState) uint {
		idx := L.CheckInt(1)
		if idx == 0 {
			return 0
		}
		if idx < 1 || idx > 9 {
			L.RaiseError("Tag index out of bounds: must be between 0 and 9")
		}
		return 1 << (idx - 1)
	}


	L.SetField(tagTable, "view", L.NewFunction(func(L *lua.LState) int {
		core.ActionView(indexToMask(L))
		return 0
	}))

	L.SetField(tagTable, "toggle_view", L.NewFunction(func(L *lua.LState) int {
		core.ActionToggleView(indexToMask(L))
		return 0
	}))

	L.SetField(tagTable, "set", L.NewFunction(func(L *lua.LState) int {
		core.ActionTag(indexToMask(L))
		return 0
	}))

	L.SetField(tagTable, "toggle", L.NewFunction(func(L *lua.LState) int {
		core.ActionToggleTag(indexToMask(L))
		return 0
	}))

	L.SetField(srwmMod, "tag", tagTable)
}
