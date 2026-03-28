package config

import (
	"github.com/infraflakes/srwm/c-src"
	lua "github.com/yuin/gopher-lua"
)

func RegisterConfigAPI(L *lua.LState, srwmMod *lua.LTable) {
	cfgTable := L.NewTable()

	L.SetField(cfgTable, "borderpx", L.NewFunction(func(L *lua.LState) int {
		return uintProp(L, core.GetBorderPx, core.SetBorderPx)
	}))

	L.SetField(srwmMod, "cfg", cfgTable)
}
