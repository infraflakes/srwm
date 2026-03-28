// Package config manages the srwm Lua runtime: it initializes the VM,
// deploys default configuration files, registers Go-backed APIs, and
// executes the user's srwmrc.lua entry point.
//
// The Lua VM runs in its own goroutine and is governed by a
// context.Context for clean cancellation during WM restarts.
package config

import (
	_ "embed"
	"fmt"
	"log"
	"os"
	"path/filepath"
)

// defaultSrwmrcScript is the embedded default srwmrc.lua, deployed to
// ~/.config/srwm/srwmrc.lua on first run.
//
//go:embed lua/srwmrc.lua
var defaultSrwmrcScript []byte

//go:embed lua/general.lua
var defaultGeneralScript []byte

//go:embed lua/canvas.lua
var defaultCanvasScript []byte

//go:embed lua/keybindings.lua
var defaultKeybindingsScript []byte

//go:embed lua/theming.lua
var defaultThemingScript []byte

//go:embed lua/bar.lua
var defaultBarScript []byte

//go:embed lua/startup.lua
var defaultStartupScript []byte

//go:embed lua/env.lua
var defaultEnvScript []byte

// defaultLuaModules maps filenames to their embedded contents.
var defaultLuaModules = map[string][]byte{
	"general.lua":     defaultGeneralScript,
	"canvas.lua":      defaultCanvasScript,
	"keybindings.lua": defaultKeybindingsScript,
	"theming.lua":     defaultThemingScript,
	"bar.lua":         defaultBarScript,
	"startup.lua":     defaultStartupScript,
	"env.lua":         defaultEnvScript,
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

	if _, err := os.Stat(rcPath); err == nil {
		return fmt.Errorf("config already exists at %s", rcPath)
	}

	if err := os.MkdirAll(srwmDir, 0755); err != nil {
		return fmt.Errorf("failed to create config dir: %w", err)
	}

	if err := os.WriteFile(rcPath, defaultSrwmrcScript, 0644); err != nil {
		return fmt.Errorf("failed to deploy srwmrc.lua: %w", err)
	}
	fmt.Printf("deployed: %s\n", rcPath)

	for name, content := range defaultLuaModules {
		path := filepath.Join(srwmDir, name)
		if err := os.WriteFile(path, content, 0644); err != nil {
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
