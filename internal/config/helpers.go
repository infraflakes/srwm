package config

import lua "github.com/yuin/gopher-lua"

func intProp(L *lua.LState, get func() int, set func(int)) int {
	if L.GetTop() == 1 {
		set(L.CheckInt(1))
	} else {
		L.Push(lua.LNumber(get()))
	}
	return 1
}

func uintProp(L *lua.LState, get func() uint, set func(uint)) int {
	if L.GetTop() == 1 {
		set(uint(L.CheckInt(1)))
	} else {
		L.Push(lua.LNumber(get()))
	}
	return 1
}

func boolProp(L *lua.LState, get func() bool, set func(bool)) int {
	if L.GetTop() == 1 {
		set(L.CheckBool(1))
	} else {
		L.Push(lua.LBool(get()))
	}
	return 1
}
