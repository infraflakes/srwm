package config

import (
	_ "embed"
	"log"
	"os"
	"path/filepath"
	"time"

	"github.com/nixuris/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

//go:embed bar.lua
var defaultBarScript []byte

// StartBar initializes the Lua VM, injects the `srwm` API, establishes
// the config file (deploying bar.lua if not found), and runs it
// in the background.
func StartBar() {
	go runLuaBar()
}

func runLuaBar() {
	// 1. Determine config path
	configDir := os.Getenv("XDG_CONFIG_HOME")
	if configDir == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			log.Printf("lua: cannot locate home dir to deploy bar.lua: %v", err)
			return
		}
		configDir = filepath.Join(home, ".config")
	}

	srwmDir := filepath.Join(configDir, "srwm")
	barPath := filepath.Join(srwmDir, "bar.lua")

	// 2. Ensure config exists
	if err := ensureConfig(srwmDir, barPath); err != nil {
		log.Printf("lua: failed to ensure config exists: %v", err)
		return
	}

	// 3. Initialize Lua VM
	L := lua.NewState()
	defer L.Close()

	// Expose srwm global module
	srwmMod := L.SetFuncs(L.NewTable(), map[string]lua.LGFunction{
		"set_status": luaSetStatus,
		"sleep":      luaSleep,
	})
	L.SetGlobal("srwm", srwmMod)

	// 4. Run the script
	log.Printf("lua: executing %s", barPath)
	if err := L.DoFile(barPath); err != nil {
		log.Printf("lua: error running %s: %v", barPath, err)
	}
}

func ensureConfig(dir, path string) error {
	if _, err := os.Stat(path); err == nil {
		return nil // exists
	}

	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}

	return os.WriteFile(path, defaultBarScript, 0644)
}

// refreshChan is used to wake up the Lua bar script from its sleep
// for immediate refreshes (e.g. after volume change).
var refreshChan = make(chan struct{}, 1)

// NotifyBarRefresh triggers an immediate refresh of the status bar
// by waking up the Lua sleep timer.
func NotifyBarRefresh() {
	select {
	case refreshChan <- struct{}{}:
	default:
		// Channel already has a pending refresh request
	}
}

func luaSetStatus(L *lua.LState) int {
	text := L.CheckString(1)
	core.SetStatus(text)
	return 0
}

func luaSleep(L *lua.LState) int {
	sec := L.CheckNumber(1)
	select {
	case <-time.After(time.Duration(float64(sec) * float64(time.Second))):
	case <-refreshChan:
	}
	return 0
}
