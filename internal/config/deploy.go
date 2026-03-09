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

//go:embed widgets/theme.sh
var widgetTheme []byte

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
	"theme.sh":      widgetTheme,
	"volume.sh":     widgetVolume,
	"wifi.sh":       widgetWifi,
}

// StartConfig boots the Lua runtime in a background goroutine.
//
// The provided context controls the VM lifetime: when ctx is cancelled
// (e.g. during a WM restart), the Lua sleep loop wakes up and the VM
// shuts down gracefully.
//
// Call this AFTER core.Init() to ensure the X11 display is ready before
// Lua tries to register keybindings.
func StartConfig(ctx context.Context) {
	go runLuaConfig(ctx)
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

	// Build the srwm.* global module table.
	srwmMod := L.NewTable()
	L.SetGlobal("srwm", srwmMod)

	RegisterBarAPI(L, srwmMod, srwmDir)
	RegisterKeybindAPI(L, srwmMod)
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
