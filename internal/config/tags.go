package config

import (
	"strings"

	"github.com/infraflakes/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

func RegisterTagsAPI(L *lua.LState, srwmMod *lua.LTable) {
	tagsTable := L.NewTable()

	tagsTable.RawSetString("set", L.NewFunction(func(L *lua.LState) int {
		input := L.CheckString(1)
		parts := strings.Split(input, ",")

		count := min(len(parts), 9)
		for i := range count {
			core.SetTag(i, strings.TrimSpace(parts[i]))
		}
		core.SetTagsLen(count)
		return 0
	}))

	L.SetField(srwmMod, "tags", tagsTable)
}
