package config

import (
	"log"
	"os"
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
	widgets   map[string]string // name → absolute script path
	layout    []string          // ordered widget names
	interval  float64           // polling interval in seconds
	configDir string            // resolved config dir for relative paths
	themePath string            // path to theme.sh (now deprecated)
	themeRepl *strings.Replacer // theme variable substitutor
	palette   map[string]string // name → hex string
}

// RegisterBarAPI creates the srwm.bar namespace and populates it with
// the shell-widget bar API:
//
//   - srwm.bar.widget(name, path) — register a named widget script
//   - srwm.bar.layout(name, ...)  — set the display order
//   - srwm.bar.interval(seconds)  — set the polling interval
//   - srwm.bar.theme({name="#hex"}) — configure string replacement variables
//   - srwm.bar.workspaces.colors("name", ...) — set tag colors from palette
//   - srwm.bar.run()              — enter the Go-managed polling loop
func RegisterBarAPI(L *lua.LState, srwmMod *lua.LTable, configDir string) {
	state := &barState{
		widgets:   make(map[string]string),
		layout:    make([]string, 0),
		interval:  1.0,
		configDir: configDir,
		themePath: filepath.Join(configDir, "widgets", "theme.sh"),
		themeRepl: strings.NewReplacer(),
		palette:   make(map[string]string),
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

	barTable.RawSetString("fonts", L.NewFunction(func(L *lua.LState) int {
		core.SetFont(L.CheckString(1))
		return 0
	}))

	barTable.RawSetString("theme", L.NewFunction(func(L *lua.LState) int {
		return luaBarTheme(L, state)
	}))

	barTable.RawSetString("run", L.NewFunction(func(L *lua.LState) int {
		return luaBarRun(L, state)
	}))

	// Nested workspaces table
	wsTable := L.NewTable()
	wsTable.RawSetString("colors", L.NewFunction(func(L *lua.LState) int {
		return luaBarWorkspacesColors(L, state)
	}))
	barTable.RawSetString("workspaces", wsTable)

	L.SetField(srwmMod, "bar", barTable)
}

// luaBarTheme registers theme variables.
func luaBarTheme(L *lua.LState, state *barState) int {
	themeTable := L.CheckTable(1)
	var replacements []string

	schemeMap := map[string]core.Scheme{
		"normal":       core.SchemeNorm,
		"selected":     core.SchemeSel,
		"title":        core.SchemeTitle,
		"tag":          core.SchemeTag,
		"tag1":         core.SchemeTag1,
		"tag2":         core.SchemeTag2,
		"tag3":         core.SchemeTag3,
		"tag4":         core.SchemeTag4,
		"tag5":         core.SchemeTag5,
		"tag6":         core.SchemeTag6,
		"tag7":         core.SchemeTag7,
		"tag8":         core.SchemeTag8,
		"tag9":         core.SchemeTag9,
		"tab_selected": core.TabSel,
		"tab_normal":   core.TabNorm,
		"button_prev":  core.SchemeBtnPrev,
		"button_next":  core.SchemeBtnNext,
		"button_close": core.SchemeBtnClose,
	}

	themeTable.ForEach(func(k, v lua.LValue) {
		key := k.String()

		// Nested table → WM core color
		if tbl, ok := v.(*lua.LTable); ok {
			scheme, exists := schemeMap[key]
			if !exists {
				return
			}
			if fg := tbl.RawGetString("fg"); fg.Type() == lua.LTString {
				core.SetColor(scheme, 0, fg.String())
			}
			if bg := tbl.RawGetString("bg"); bg.Type() == lua.LTString {
				core.SetColor(scheme, 1, bg.String())
			}
			if border := tbl.RawGetString("border"); border.Type() == lua.LTString {
				core.SetColor(scheme, 2, border.String())
			}
			return
		}

		// Simple string → widget template replacement AND palette storage
		color := v.String()
		state.palette[key] = color
		replacements = append(replacements, "{"+key+"}", color)
	})

	state.themeRepl = strings.NewReplacer(replacements...)
	return 0
}

func luaBarWorkspacesColors(L *lua.LState, state *barState) int {
	n := L.GetTop()
	for i := 1; i <= n && i <= 9; i++ {
		colorName := L.CheckString(i)
		if hex, ok := state.palette[colorName]; ok {
			// Map tag i-1 to SchemeTagX
			// We have SchemeTag1 through SchemeTag9
			scheme := core.SchemeTag1 + core.Scheme(i-1)
			if scheme > core.SchemeTag9 {
				scheme = core.SchemeTag9
			}

			// Set foreground for this tag scheme
			core.SetColor(scheme, 0, hex)
			// Map the tag to use this scheme
			core.SetTagScheme(i-1, scheme)
		}
	}
	return 0
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
// replaces theme variables via template string expansion,
// and calls core.SetStatus().
//
// The loop exits when the Lua VM's context is cancelled.
// This function blocks the calling Lua thread.
func luaBarRun(L *lua.LState, state *barState) int {
	// Signal that all config values have been set
	select {
	case configReady <- struct{}{}:
	default:
	}

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
			// Avoid propagating ugly env vars to the widgets.
			cmd.Env = os.Environ()

			out, err := cmd.CombinedOutput()
			if err != nil {
				log.Printf("srwm.bar: widget %q failed: %v (output: %s)", name, err, string(out))
				continue
			}

			text := strings.TrimRight(string(out), "\n\r")
			if text != "" {
				parts = append(parts, text)
			}
		}

		finalStatus := strings.Join(parts, "")
		finalStatus = state.themeRepl.Replace(finalStatus)
		core.SetStatus(finalStatus)

		// Interruptible sleep
		select {
		case <-time.After(time.Duration(state.interval * float64(time.Second))):
		case <-refreshChan:
		case <-L.Context().Done():
			return 0
		}
	}
}
