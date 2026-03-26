package config

import (
	"strings"

	"github.com/infraflakes/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

func RegisterWorkspacesAPI(L *lua.LState, srwmMod *lua.LTable) {
	wsTable := L.NewTable()

	wsTable.RawSetString("set_label", L.NewFunction(func(L *lua.LState) int {
		input := L.CheckString(1)
		parts := strings.Split(input, ",")

		count := min(len(parts), 9)
		for i := range count {
			core.SetWsLabel(i, strings.TrimSpace(parts[i]))
		}
		core.SetWsCount(count)
		return 0
	}))

	L.SetField(srwmMod, "workspaces", wsTable)
}
