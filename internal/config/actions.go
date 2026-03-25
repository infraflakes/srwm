package config

import (
	"github.com/infraflakes/srwm/internal/core"
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

	L.SetField(tagTable, "view", L.NewFunction(func(L *lua.LState) int {
		idx := L.CheckInt(1)
		core.ActionView(idx - 1) // Lua 1-based to C 0-based
		return 0
	}))

	L.SetField(tagTable, "move_window_to", L.NewFunction(func(L *lua.LState) int {
		idx := L.CheckInt(1)
		core.ActionTag(idx - 1) // Lua 1-based to C 0-based
		return 0
	}))

	L.SetField(srwmMod, "tag", tagTable)
	// srwm.canvas namespace
	canvasTable := L.NewTable()

	L.SetField(canvasTable, "move", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionMoveCanvas(dir)
		return 0
	}))

	L.SetField(canvasTable, "home", L.NewFunction(func(L *lua.LState) int {
		core.ActionHomeCanvas()
		return 0
	}))

	L.SetField(canvasTable, "center_window", L.NewFunction(func(L *lua.LState) int {
		core.ActionCenterWindowOnCanvas()
		return 0
	}))

	L.SetField(canvasTable, "zoom", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionZoomCanvas(dir)
		return 0
	}))

	L.SetField(srwmMod, "canvas", canvasTable)
}
