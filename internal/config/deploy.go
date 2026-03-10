// Package config manages the srwm Lua runtime: it initializes the VM,
// deploys default configuration files, registers Go-backed APIs, and
// executes the user's srwmrc.lua entry point.
//
// The Lua VM runs in its own goroutine and is governed by a
// context.Context for clean cancellation during WM restarts.
package config

import (
	"context"
	_ "embed"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/nixuris/srwm/internal/control"
	lua "github.com/yuin/gopher-lua"
)

// defaultSrwmrcScript is the embedded default srwmrc.lua, deployed to
// ~/.config/srwm/srwmrc.lua on first run.
//
//go:embed lua/srwmrc.lua
var defaultSrwmrcScript []byte

//go:embed widgets/battery.sh
var widgetBattery []byte

//go:embed widgets/brightness.sh
var widgetBrightness []byte

//go:embed widgets/clock.sh
var widgetClock []byte

//go:embed widgets/gap.sh
var widgetGap []byte

//go:embed widgets/volume.sh
var widgetVolume []byte

//go:embed widgets/wifi.sh
var widgetWifi []byte

// defaultWidgets maps filenames to their embedded contents.
var defaultWidgets = map[string][]byte{
	"battery.sh":    widgetBattery,
	"brightness.sh": widgetBrightness,
	"clock.sh":      widgetClock,
	"gap.sh":        widgetGap,
	"volume.sh":     widgetVolume,
	"wifi.sh":       widgetWifi,
}

// configReady is signalled by the Lua goroutine once all config setters
// (fonts, colors, padding, etc.) have run. start.go waits on this
// before calling core.InitSetup() so setup() reads the correct values.
var configReady = make(chan struct{}, 1)

// StartConfig boots the Lua runtime in a background goroutine.
// Call this AFTER core.InitDisplay().
func StartConfig(ctx context.Context) {
	configReady = make(chan struct{}, 1)
	go runLuaConfig(ctx)
}

// WaitConfigReady blocks until the Lua config has finished setting values.
// Call this BEFORE core.InitSetup().
func WaitConfigReady() {
	<-configReady
}

// runLuaConfig is the main Lua configuration goroutine. It:
//  1. Resolves the config directory (~/.config/srwm/).
//  2. Deploys default scripts if missing.
//  3. Creates and configures the Lua VM.
//  4. Registers the srwm.* API surface.
//  5. Executes srwmrc.lua (which blocks in the bar loop).
func runLuaConfig(ctx context.Context) {
	srwmDir := ResolveConfigDir()
	if srwmDir == "" {
		return
	}

	rcPath := filepath.Join(srwmDir, "srwmrc.lua")
	widgetsDir := filepath.Join(srwmDir, "widgets")

	if err := os.MkdirAll(widgetsDir, 0755); err != nil {
		log.Printf("lua: failed to create config/widgets dir: %v", err)
		return
	}

	// Deploy defaults (does not overwrite existing files)
	deployDefault(rcPath, defaultSrwmrcScript, 0644)
	for name, content := range defaultWidgets {
		deployDefault(filepath.Join(widgetsDir, name), content, 0755)
	}

	L := lua.NewState()
	L.OpenLibs()
	L.SetContext(ctx)
	defer L.Close()

	L.SetGlobal("include", L.NewFunction(func(L *lua.LState) int {
		path := L.CheckString(1)
		if !strings.HasSuffix(path, ".lua") {
			path = path + ".lua"
		}
		var fullPath string
		if filepath.IsAbs(path) {
			fullPath = path
		} else {
			fullPath = filepath.Join(srwmDir, path)
		}
		if err := L.DoFile(fullPath); err != nil {
			L.RaiseError("%s", "include "+path+": "+err.Error())
		}
		return 0
	}))

	// Build the srwm.* global module table.
	srwmMod := L.NewTable()
	L.SetGlobal("srwm", srwmMod)

	RegisterBarAPI(L, srwmMod, srwmDir)
	RegisterKeybindAPI(L, srwmMod)
	RegisterConfigAPI(L, srwmMod)
	control.RegisterAPI(L, srwmMod)

	log.Printf("lua: executing %s", rcPath)
	if err := L.DoFile(rcPath); err != nil {
		log.Printf("lua: error running %s: %v", rcPath, err)
	}
}

// ResolveConfigDir returns the srwm config directory path
// (~/.config/srwm), respecting XDG_CONFIG_HOME.
func ResolveConfigDir() string {
	configDir := os.Getenv("XDG_CONFIG_HOME")
	if configDir == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			log.Printf("lua: cannot locate home dir: %v", err)
			return ""
		}
		configDir = filepath.Join(home, ".config")
	}
	return filepath.Join(configDir, "srwm")
}

// DeployKickstart forcefully deploys default scripts to the config directory.
// It returns an error if a config file already exists to prevent overwriting
// user configurations.
func DeployKickstart() error {
	srwmDir := ResolveConfigDir()
	if srwmDir == "" {
		return fmt.Errorf("failed to resolve config directory")
	}

	rcPath := filepath.Join(srwmDir, "srwmrc.lua")
	widgetsDir := filepath.Join(srwmDir, "widgets")

	if _, err := os.Stat(rcPath); err == nil {
		return fmt.Errorf("config already exists at %s", rcPath)
	}

	if err := os.MkdirAll(widgetsDir, 0755); err != nil {
		return fmt.Errorf("failed to create config/widgets dir: %w", err)
	}

	if err := os.WriteFile(rcPath, defaultSrwmrcScript, 0644); err != nil {
		return fmt.Errorf("failed to deploy srwmrc.lua: %w", err)
	}
	fmt.Printf("deployed: %s\n", rcPath)

	for name, content := range defaultWidgets {
		path := filepath.Join(widgetsDir, name)
		if err := os.WriteFile(path, content, 0755); err != nil {
			return fmt.Errorf("failed to deploy %s: %w", name, err)
		}
		fmt.Printf("deployed: %s\n", path)
	}

	return nil
}

// deployDefault writes content to path only if the file does not already
// exist. This is used to deploy default config files on first run without
// overwriting user edits.
func deployDefault(path string, content []byte, perm os.FileMode) {
	if _, err := os.Stat(path); err == nil {
		return // already exists, don't overwrite
	}
	if err := os.WriteFile(path, content, perm); err != nil {
		log.Printf("lua: failed to deploy %s: %v", path, err)
	}
}
