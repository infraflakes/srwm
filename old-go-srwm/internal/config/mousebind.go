package config

import (
	"log"
	"strings"

	"github.com/infraflakes/srwm/c-src"
	lua "github.com/yuin/gopher-lua"
)

// Button name to X11 button number mapping
var buttonAliases = map[string]uint{
	"button1": 1, "left": 1,
	"button2": 2, "middle": 2,
	"button3": 3, "right": 3,
	"button4": 4, "scrollup": 4,
	"button5": 5, "scrolldown": 5,
}

// Click target name to enum mapping
var clickAliases = map[string]uint{
	"root":   core.ClkRootWin,
	"client": core.ClkClientWin,
	"tabbar": core.ClkTabBar,
}

func RegisterMousebindAPI(L *lua.LState, srwmMod *lua.LTable) {
	mouseTable := L.NewTable()
	L.SetField(mouseTable, "bind", L.NewFunction(luaMouseBind))
	L.SetField(srwmMod, "mouse", mouseTable)
}

// Lua signature: srwm.mouse.bind(modifiers: string, button: string, target: string, callback: function)
// Example: srwm.mouse.bind("Mod4", "ScrollUp", "root", function() srwm.canvas.zoom(1) end)
func luaMouseBind(L *lua.LState) int {
	modStr := L.CheckString(1)
	buttonName := L.CheckString(2)
	targetName := L.CheckString(3)
	callback := L.CheckFunction(4)

	mod := parseModifiers(modStr)

	btnName := strings.ToLower(buttonName)
	btn, ok := buttonAliases[btnName]
	if !ok {
		L.RaiseError("srwm.mouse.bind: invalid button name %q", buttonName)
		return 0
	}

	tgtName := strings.ToLower(targetName)
	click, ok := clickAliases[tgtName]
	if !ok {
		L.RaiseError("srwm.mouse.bind: invalid target %q (expected root, client, tabbar)", targetName)
		return 0
	}

	co, _ := L.NewThread()
	goCb := func() {
		err := L.CallByParam(lua.P{
			Fn:      callback,
			NRet:    0,
			Protect: true,
		}, co)
		if err != nil {
			log.Printf("srwm.mouse callback error: %v", err)
		}
	}

	core.AddMouseBinding(click, uint(mod), btn, goCb)
	return 0
}
