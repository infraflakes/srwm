package config

import (
	"log"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/nixuris/srwm/internal/core"
	lua "github.com/yuin/gopher-lua"
)

// refreshChan is used to wake up the Go-managed bar polling loop for
// immediate status bar refreshes (e.g. after a volume change).
var refreshChan = make(chan struct{}, 1)

// NotifyBarRefresh triggers an immediate refresh of the status bar.
//
// Safe to call from any goroutine. If a refresh is already pending
// it is a no-op (the channel is buffered with capacity 1).
func NotifyBarRefresh() {
	select {
	case refreshChan <- struct{}{}:
	default:
	}
}

// barState holds per-VM bar configuration set by Lua before run().
type barState struct {
	widgets  map[string]string // name → absolute script path
	layout   []string          // ordered widget names
	interval float64           // polling interval in seconds
	configDir string           // resolved config dir for relative paths
	themePath string           // path to theme.sh
}

// RegisterBarAPI creates the srwm.bar namespace and populates it with
// the shell-widget bar API:
//
//   - srwm.bar.widget(name, path) — register a named widget script
//   - srwm.bar.layout(name, ...)  — set the display order
//   - srwm.bar.interval(seconds)  — set the polling interval
//   - srwm.bar.separator(name)    — set the separator widget name
//   - srwm.bar.run()              — enter the Go-managed polling loop
func RegisterBarAPI(L *lua.LState, srwmMod *lua.LTable, configDir string) {
	state := &barState{
		widgets:   make(map[string]string),
		interval:  1.0,
		configDir: configDir,
		themePath: filepath.Join(configDir, "widgets", "theme.sh"),
	}

	barTable := L.NewTable()

	barTable.RawSetString("widget", L.NewFunction(func(L *lua.LState) int {
		return luaBarWidget(L, state)
	}))

	barTable.RawSetString("layout", L.NewFunction(func(L *lua.LState) int {
		return luaBarLayout(L, state)
	}))

	barTable.RawSetString("interval", L.NewFunction(func(L *lua.LState) int {
		state.interval = float64(L.CheckNumber(1))
		return 0
	}))

	barTable.RawSetString("run", L.NewFunction(func(L *lua.LState) int {
		return luaBarRun(L, state)
	}))

	L.SetField(srwmMod, "bar", barTable)
}

// luaBarWidget registers a named widget pointing to a shell script.
//
// Lua signature: srwm.bar.widget(name: string, path: string)
//
// Relative paths are resolved from ~/.config/srwm/.
func luaBarWidget(L *lua.LState, state *barState) int {
	name := L.CheckString(1)
	scriptPath := L.CheckString(2)

	// Resolve relative paths from the config directory
	if !filepath.IsAbs(scriptPath) {
		scriptPath = filepath.Join(state.configDir, scriptPath)
	}

	state.widgets[name] = scriptPath
	return 0
}

// luaBarLayout sets the display order of widgets.
//
// Lua signature: srwm.bar.layout(name1, name2, ...)
//
// All arguments are widget names that must have been registered via
// srwm.bar.widget(). Unknown names are silently skipped at render time.
func luaBarLayout(L *lua.LState, state *barState) int {
	n := L.GetTop()
	state.layout = make([]string, 0, n)
	for i := 1; i <= n; i++ {
		state.layout = append(state.layout, L.CheckString(i))
	}
	return 0
}

// luaBarRun enters the Go-managed polling loop.
//
// On each tick (or instant refresh), Go iterates over the layout,
// executes each widget's shell script, concatenates the output,
// and calls core.SetStatus().
//
// The SRWM_THEME environment variable is set to the theme.sh path
// so widget scripts can source it for color variables.
//
// The loop exits when the Lua VM's context is cancelled.
// This function blocks the calling Lua thread.
func luaBarRun(L *lua.LState, state *barState) int {
	if len(state.layout) == 0 {
		log.Println("srwm.bar.run: no layout defined, nothing to render")
		return 0
	}

	for {
		var parts []string
		for _, name := range state.layout {
			scriptPath, ok := state.widgets[name]
			if !ok {
				continue
			}

			cmd := exec.Command("sh", scriptPath)
			cmd.Env = append(cmd.Environ(), "SRWM_THEME="+state.themePath)

			out, err := cmd.Output()
			if err != nil {
				continue // skip failed widgets silently
			}

			text := strings.TrimRight(string(out), "\n\r")
			if text != "" {
				parts = append(parts, text)
			}
		}

		core.SetStatus(strings.Join(parts, ""))

		// Interruptible sleep
		select {
		case <-time.After(time.Duration(state.interval * float64(time.Second))):
		case <-refreshChan:
		case <-L.Context().Done():
			return 0
		}
	}
}
