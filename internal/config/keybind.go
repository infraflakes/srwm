package config

import (
	"log"
	"strings"

	"github.com/infraflakes/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

// X11 modifier mask constants.
// These mirror the X11 <X11/X.h> definitions and are exposed to Lua
// as srwm.Mod1, srwm.Mod4, etc. for use with srwm.key.set().
const (
	ShiftMask   = 1 << 0 // 1  — Shift key
	ControlMask = 1 << 2 // 4  — Ctrl key
	Mod1Mask    = 1 << 3 // 8  — Alt / Meta
	Mod4Mask    = 1 << 6 // 64 — Super / Win
)

// modifierAliases maps human-readable modifier names (lowercase) to
// their X11 mask values. Used by parseModifiers to convert strings
// like "Mod4+Shift" into the corresponding bitmask.
var modifierAliases = map[string]int{
	"shift":   ShiftMask,
	"ctrl":    ControlMask,
	"control": ControlMask,
	"mod1":    Mod1Mask,
	"alt":     Mod1Mask,
	"mod4":    Mod4Mask,
	"super":   Mod4Mask,
	"win":     Mod4Mask,
}

// RegisterKeybindAPI injects key-related functions and modifier
// constants into the srwm Lua module table:
//
//   - srwm.key.bind(mods, key, fn) — register a keybinding using
//     human-readable strings (e.g. "Mod4+Shift", "Return").
//   - srwm.key.set(mod, keysym, fn) — register a keybinding using
//     raw numeric modifier and keysym values.
//   - srwm.Mod1, srwm.Mod4, srwm.Shift, srwm.Ctrl — modifier
//     constants for use with srwm.key.set().
func RegisterKeybindAPI(L *lua.LState, srwmMod *lua.LTable) {
	// Expose raw modifier constants for srwm.key.set().
	L.SetField(srwmMod, "Mod1", lua.LNumber(Mod1Mask))
	L.SetField(srwmMod, "Mod4", lua.LNumber(Mod4Mask))
	L.SetField(srwmMod, "Shift", lua.LNumber(ShiftMask))
	L.SetField(srwmMod, "Ctrl", lua.LNumber(ControlMask))

	// Build the srwm.key namespace table.
	keyTable := L.NewTable()
	L.SetField(keyTable, "bind", L.NewFunction(luaKeyBind))
	L.SetField(keyTable, "set", L.NewFunction(luaKeySet))
	L.SetField(srwmMod, "key", keyTable)
}

// luaKeyBind registers a dynamic keybinding using human-readable strings.
//
// Lua signature: srwm.key.bind(modifiers: string, keyname: string, callback: function)
//
// Examples:
//
//	srwm.key.bind("Mod4+Shift", "Return", function() ... end)
//	srwm.key.bind("",           "XF86AudioMute", function() ... end)
//
// The modifier string is a "+"-separated list of modifier names
// (case-insensitive). Supported names: Mod1, Alt, Mod4, Super, Win,
// Shift, Ctrl, Control. An empty string means no modifier.
//
// The key name is passed to XStringToKeysym for resolution, so any
// standard X11 keysym name is valid (e.g. "a", "Return", "space",
// "XF86AudioRaiseVolume").
func luaKeyBind(L *lua.LState) int {
	modStr := L.CheckString(1)
	keyName := L.CheckString(2)
	callback := L.CheckFunction(3)

	mod := parseModifiers(modStr)
	keysym := core.StringToKeysym(keyName)

	if keysym == 0 {
		L.RaiseError("srwm.key.bind: invalid key name %q (XStringToKeysym returned NoSymbol)", keyName)
		return 0
	}

	registerBinding(L, uint(mod), keysym, callback)
	return 0
}

// luaKeySet registers a dynamic keybinding using raw numeric values.
//
// Lua signature: srwm.key.set(modifier: number, keysym: number, callback: function)
//
// This is the low-level API. Most users should prefer srwm.key.bind().
func luaKeySet(L *lua.LState) int {
	mod := L.CheckNumber(1)
	keysym := L.CheckNumber(2)
	callback := L.CheckFunction(3)

	registerBinding(L, uint(mod), uint(keysym), callback)
	return 0
}

// registerBinding is the shared implementation for luaKeyBind and
// luaKeySet. It creates a Go closure that will invoke the Lua callback
// in a coroutine, registers the keybinding in the C core, and
// immediately re-grabs all keys so the binding takes effect.
func registerBinding(L *lua.LState, mod uint, keysym uint, callback *lua.LFunction) {
	co, _ := L.NewThread()

	goCb := func() {
		err := L.CallByParam(lua.P{
			Fn:      callback,
			NRet:    0,
			Protect: true,
		}, co)
		if err != nil {
			log.Printf("srwm.key callback error: %v", err)
		}
	}

	core.AddKeybinding(mod, keysym, goCb)
	core.GrabKeys()
}

// parseModifiers converts a "+"-separated modifier string into the
// corresponding X11 modifier bitmask.
//
// Examples:
//
//	parseModifiers("Mod4+Shift")  → Mod4Mask | ShiftMask = 65
//	parseModifiers("")            → 0 (no modifiers)
//	parseModifiers("Alt+Ctrl")   → Mod1Mask | ControlMask = 12
func parseModifiers(s string) int {
	var mask int
	for part := range strings.SplitSeq(s, "+") {
		name := strings.TrimSpace(strings.ToLower(part))
		if val, ok := modifierAliases[name]; ok {
			mask |= val
		}
	}
	return mask
}
