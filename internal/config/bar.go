package config

import (
	"github.com/infraflakes/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

func NotifyBarRefresh() {
}

func RegisterBarAPI(L *lua.LState, srwmMod *lua.LTable, configDir string) func() {
	barTable := L.NewTable()

	barTable.RawSetString("fonts", L.NewFunction(func(L *lua.LState) int {
		core.SetFont(L.CheckString(1))
		return 0
	}))

	barTable.RawSetString("tab_height", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabHeight, core.SetTabHeight)
	}))

	barTable.RawSetString("tab_tile_vertical_padding", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabTileVerticalPadding, core.SetTabTileVerticalPadding)
	}))

	barTable.RawSetString("tab_tile_outer_padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabTileOuterPaddingHorizontal, core.SetTabTileOuterPaddingHorizontal)
	}))

	barTable.RawSetString("tab_tile_inner_padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabTileInnerPaddingHorizontal, core.SetTabTileInnerPaddingHorizontal)
	}))

	barTable.RawSetString("tab_top", L.NewFunction(func(L *lua.LState) int {
		return boolProp(L, core.GetTopTab, core.SetTopTab)
	}))

	barTable.RawSetString("theme", L.NewFunction(func(L *lua.LState) int {
		return luaBarTheme(L)
	}))

	L.SetField(srwmMod, "bar", barTable)

	return func() {}
}

func luaBarTheme(L *lua.LState) int {
	themeTable := L.CheckTable(1)

	twoColorSchemes := map[string]core.Scheme{
		"tab_selected": core.TabSel,
		"tab_normal":   core.TabNorm,
	}

	oneColorSchemes := map[string]core.Scheme{
		"button_prev":  core.SchemeBtnPrev,
		"button_next":  core.SchemeBtnNext,
		"button_close": core.SchemeBtnClose,
	}

	themeTable.ForEach(func(k, v lua.LValue) {
		key := k.String()

		if tbl, ok := v.(*lua.LTable); ok {
			scheme, exists := twoColorSchemes[key]
			if !exists {
				return
			}
			if fg := tbl.RawGetInt(1); fg.Type() == lua.LTString {
				core.SetColor(scheme, 0, fg.String())
			}
			if bg := tbl.RawGetInt(2); bg.Type() == lua.LTString {
				core.SetColor(scheme, 1, bg.String())
			}
			return
		}

		if str, ok := v.(lua.LString); ok {
			hex := string(str)
			if scheme, exists := oneColorSchemes[key]; exists {
				core.SetColor(scheme, 0, hex)
			}
		}
	})

	return 0
}
