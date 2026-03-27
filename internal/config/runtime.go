package config

import (
	"context"
	"log"
	"path/filepath"
	"strings"

	"github.com/infraflakes/srwm/internal/control"
	lua "github.com/yuin/gopher-lua"
)

var configReady = make(chan struct{}, 1)

func StartConfig(ctx context.Context) {
	configReady = make(chan struct{}, 1)
	go runLuaConfig(ctx)
}

func WaitConfigReady() {
	<-configReady
}

func runLuaConfig(ctx context.Context) {
	srwmDir := ResolveConfigDir()
	if srwmDir == "" {
		return
	}

	rcPath := filepath.Join(srwmDir, "srwmrc.lua")

	deployDefault(rcPath, defaultSrwmrcScript, 0644)
	for name, content := range defaultLuaModules {
		deployDefault(filepath.Join(srwmDir, name), content, 0644)
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
			log.Printf("lua: include %s: %v", path, err)
		}
		return 0
	}))

	srwmMod := L.NewTable()
	L.SetGlobal("srwm", srwmMod)

	startBar := RegisterBarAPI(L, srwmMod, srwmDir)
	RegisterKeybindAPI(L, srwmMod)
	RegisterMousebindAPI(L, srwmMod)
	RegisterConfigAPI(L, srwmMod)
	RegisterActionsAPI(L, srwmMod)
	control.RegisterAPI(L, srwmMod)

	RegisterWorkspacesAPI(L, srwmMod)

	log.Printf("lua: executing %s", rcPath)
	if err := L.DoFile(rcPath); err != nil {
		log.Printf("lua: error running %s: %v", rcPath, err)
	}

	startBar()

	select {
	case configReady <- struct{}{}:
	default:
	}

	<-ctx.Done()
}
