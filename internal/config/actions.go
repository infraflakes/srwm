package config

import (
	"github.com/infraflakes/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

// RegisterActionsAPI sets up the srwm.window and srwm.workspace namespaces.
func RegisterActionsAPI(L *lua.LState, srwmMod *lua.LTable) {
	// srwm.window namespace
	windowTable := L.NewTable()

	L.SetField(windowTable, "kill", L.NewFunction(func(L *lua.LState) int {
		core.ActionKillClient()
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

	// srwm.workspace namespace
	wsTable := L.NewTable()

	L.SetField(wsTable, "view_prev", L.NewFunction(func(L *lua.LState) int {
		core.ActionWsToPrev()
		return 0
	}))

	L.SetField(wsTable, "view_next", L.NewFunction(func(L *lua.LState) int {
		core.ActionWsToNext()
		return 0
	}))

	L.SetField(wsTable, "shift_view", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionShiftWs(dir)
		return 0
	}))

	L.SetField(wsTable, "move_to_monitor", L.NewFunction(func(L *lua.LState) int {
		dir := L.CheckInt(1)
		core.ActionMoveWindowToMonitor(dir)
		return 0
	}))

	L.SetField(wsTable, "view", L.NewFunction(func(L *lua.LState) int {
		idx := L.CheckInt(1)
		core.ActionView(idx - 1) // Lua 1-based to C 0-based
		return 0
	}))

	L.SetField(wsTable, "move_window_to", L.NewFunction(func(L *lua.LState) int {
		idx := L.CheckInt(1)
		core.ActionMoveToWs(idx - 1) // Lua 1-based to C 0-based
		return 0
	}))

	L.SetField(srwmMod, "workspace", wsTable)
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
