// Package control provides Lua-facing lifecycle and process management
// functions for the srwm window manager.
//
// This package exposes srwm.spawn, srwm.restart, and srwm.quit to Lua.
// These are "WM control" operations that are orthogonal to the status bar
// and keybinding subsystems, so they live in their own package to keep
// responsibilities cleanly separated.
package control

import (
	"log"
	"os"
	"os/exec"
	"sync"

	"github.com/infraflakes/srwm/c-src"
	lua "github.com/yuin/gopher-lua"
)

// spawnedOnce tracks commands that have already been executed via
// spawn_once during the lifetime of the Go process.
var spawnedOnce sync.Map

// refreshNotifier is a callback set by the config package to trigger a
// bar refresh after a spawn completes. This avoids a direct dependency
// on internal/config from this package.
var refreshNotifier func()

// SetRefreshNotifier registers a callback that is invoked after every
// srwm.spawn command completes. Typically wired to config.NotifyBarRefresh.
func SetRefreshNotifier(fn func()) {
	refreshNotifier = fn
}

// RegisterAPI injects lifecycle and process management functions into
// the srwm Lua module table:
//
//   - srwm.spawn(cmd)        — run a shell command in the background
//   - srwm.spawn_once(cmd)   — like spawn, but only runs once per WM lifetime
//   - srwm.restart()         — signal the WM to soft-restart (reload config)
//   - srwm.quit()            — signal the WM to exit cleanly
//   - srwm.env(key [, value]) — get or set an environment variable

func RegisterAPI(L *lua.LState, srwmMod *lua.LTable) {
	L.SetField(srwmMod, "spawn", L.NewFunction(luaSpawn))
	L.SetField(srwmMod, "spawn_once", L.NewFunction(luaSpawnOnce))
	L.SetField(srwmMod, "restart", L.NewFunction(luaRestart))
	L.SetField(srwmMod, "quit", L.NewFunction(luaQuit))
	L.SetField(srwmMod, "env", L.NewFunction(luaEnv))
}

// luaSpawnOnce runs a shell command asynchronously, only if it hasn't
// already been run during the lifetime of the WM process.
//
// Lua signature: srwm.spawn_once(cmd: string)
//
// This is ideal for startup applications (compositor, wallpaper) that
// should not be relaunched when the WM soft-restarts.
func luaSpawnOnce(L *lua.LState) int {
	cmd := L.CheckString(1)
	if _, loaded := spawnedOnce.LoadOrStore(cmd, true); loaded {
		return 0 // Already spawned in this WM lifetime
	}

	go func() {
		out, err := exec.Command("sh", "-c", cmd).CombinedOutput()
		if err != nil {
			log.Printf("srwm.spawn_once: %q failed: %v (output: %s)", cmd, err, string(out))
		}
	}()
	return 0
}

// luaSpawn runs a shell command asynchronously.
//
// Lua signature: srwm.spawn(cmd: string)
//
// The command is executed in a background goroutine via "sh -c <cmd>".
// After the command exits (success or failure), the bar refresh callback
// is invoked so the status bar can immediately reflect any changes
// (e.g. updated volume level after wpctl).
func luaSpawn(L *lua.LState) int {
	cmd := L.CheckString(1)
	go func() {
		out, err := exec.Command("sh", "-c", cmd).CombinedOutput()
		if err != nil {
			log.Printf("srwm.spawn: %q failed: %v (output: %s)", cmd, err, out)
		}
		if refreshNotifier != nil {
			refreshNotifier()
		}
	}()
	return 0
}

// luaRestart signals the C core to perform a soft restart.
//
// Lua signature: srwm.restart()
//
// This sets the restart flag and stops the X11 event loop. The Go
// lifecycle loop in cmd/start.go will then re-init the C core and
// reload the Lua configuration.
func luaRestart(L *lua.LState) int {
	core.Restart()
	return 0
}

// luaQuit signals the C core to cleanly exit.
//
// Lua signature: srwm.quit()
//
// This stops the X11 event loop without setting the restart flag,
// causing the Go lifecycle loop to break and the process to terminate.
func luaQuit(L *lua.LState) int {
	core.Quit()
	return 0
}

// luaEnv gets or sets an environment variable.
//
// Lua signature:
//
//	srwm.env("KEY", "VALUE")  — set an environment variable
//	srwm.env("KEY")           — get an environment variable (returns value or nil)
func luaEnv(L *lua.LState) int {
	key := L.CheckString(1)
	if L.GetTop() >= 2 {
		value := L.CheckString(2)
		if err := os.Setenv(key, value); err != nil {
			L.RaiseError("srwm.env: failed to set %s: %v", key, err)
		}
		return 0
	}
	// Getter: return current value or nil
	value, ok := os.LookupEnv(key)
	if !ok {
		L.Push(lua.LNil)
	} else {
		L.Push(lua.LString(value))
	}
	return 1
}
