package config

import (
	"context"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/infraflakes/srwm/internal/core"
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
	themeRepl *strings.Replacer // theme variable substitutor
	palette   map[string]string // name → hex string
}

// RegisterBarAPI creates the srwm.bar namespace and returns a function to start
// the background bar polling loop.
func RegisterBarAPI(L *lua.LState, srwmMod *lua.LTable, configDir string) func() {
	state := &barState{
		widgets:   make(map[string]string),
		layout:    make([]string, 0),
		interval:  1.0,
		configDir: configDir,
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

	barTable.RawSetString("padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetBarHorizontalPadding, core.SetBarHorizontalPadding)
	}))

	barTable.RawSetString("padding_vertical", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetBarVerticalPadding, core.SetBarVerticalPadding)
	}))

	barTable.RawSetString("bg", L.NewFunction(func(L *lua.LState) int {
		core.SetBarBg(L.CheckString(1))
		return 0
	}))

	barTable.RawSetString("tab_height", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabHeight, core.SetTabHeight)
	}))

	barTable.RawSetString("tab_tile_vertical_padding", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabTileVerticalPadding, core.SetTabTileVerticalPadding)
	}))

	barTable.RawSetString("tab_tile_outer_padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabTileOuterPaddingHorizontal, core.SetTabTileOuterPaddingHorizontal)
	}))

	barTable.RawSetString("tab_tile_inner_padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		return intProp(L, core.GetTabTileInnerPaddingHorizontal, core.SetTabTileInnerPaddingHorizontal)
	}))

	barTable.RawSetString("show", L.NewFunction(func(L *lua.LState) int {
		return boolProp(L, core.GetShowbar, core.SetShowbar)
	}))

	barTable.RawSetString("top", L.NewFunction(func(L *lua.LState) int {
		return boolProp(L, core.GetTopBar, core.SetTopBar)
	}))

	barTable.RawSetString("tab_top", L.NewFunction(func(L *lua.LState) int {
		return boolProp(L, core.GetTopTab, core.SetTopTab)
	}))

	barTable.RawSetString("systray", L.NewFunction(func(L *lua.LState) int {
		return boolProp(L, core.GetSystrayEnable, core.SetSystrayEnable)
	}))

	barTable.RawSetString("systray_spacing", L.NewFunction(func(L *lua.LState) int {
		return uintProp(L, core.GetSystraySpacing, core.SetSystraySpacing)
	}))

	barTable.RawSetString("systray_pinning", L.NewFunction(func(L *lua.LState) int {
		return uintProp(L, core.GetSystrayPinning, core.SetSystrayPinning)
	}))

	barTable.RawSetString("ws_underline_padding", L.NewFunction(func(L *lua.LState) int {
		return uintProp(L, core.GetWorkspaceUnderlinePadding, core.SetWorkspaceUnderlinePadding)
	}))

	barTable.RawSetString("ws_underline_size", L.NewFunction(func(L *lua.LState) int {
		return uintProp(L, core.GetWorkspaceUnderlineSize, core.SetWorkspaceUnderlineSize)
	}))

	barTable.RawSetString("ws_underline_offset", L.NewFunction(func(L *lua.LState) int {
		return uintProp(L, core.GetWorkspaceUnderlineOffsetFromBarBottom, core.SetWorkspaceUnderlineOffsetFromBarBottom)
	}))

	barTable.RawSetString("ws_underline_all_workspaces", L.NewFunction(func(L *lua.LState) int {
		return boolProp(L, core.GetWorkspaceUnderlineForAllWorkspaces, core.SetWorkspaceUnderlineForAllWorkspaces)
	}))

	barTable.RawSetString("theme", L.NewFunction(func(L *lua.LState) int {
		return luaBarTheme(L, state)
	}))

	// Nested workspaces table
	wsTable := L.NewTable()
	wsTable.RawSetString("highlight_occupied_only", L.NewFunction(func(L *lua.LState) int {
		core.SetWorkspaceColorfulOccupiedOnly(L.CheckBool(1))
		return 0
	}))
	barTable.RawSetString("workspaces", wsTable)

	L.SetField(srwmMod, "bar", barTable)

	// Return the starter function that will be called by deploy.go
	return func() {
		if len(state.layout) == 0 {
			log.Println("srwm.bar: no layout defined, skipping polling loop")
			return
		}

		// Start the polling loop in a background goroutine.
		go func(ctx context.Context) {
			for {
				var parts []string
				for _, name := range state.layout {
					scriptPath, ok := state.widgets[name]
					if !ok {
						continue
					}

					cmd := exec.Command("sh", scriptPath)
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
				case <-ctx.Done():
					return
				}
			}
		}(L.Context())
	}
}

// luaBarTheme registers theme variables.
func luaBarTheme(L *lua.LState, state *barState) int {
	themeTable := L.CheckTable(1)
	var replacements []string

	// 2-color schemes: {fg, bg}
	twoColorSchemes := map[string]struct {
		scheme core.Scheme
		wsIdx  int
	}{
		"title":        {core.SchemeTitle, -1},
		"inactive_ws":  {core.SchemeWorkspace, -1},
		"ws_1":         {core.SchemeWorkspace1, 0},
		"ws_2":         {core.SchemeWorkspace2, 1},
		"ws_3":         {core.SchemeWorkspace3, 2},
		"ws_4":         {core.SchemeWorkspace4, 3},
		"ws_5":         {core.SchemeWorkspace5, 4},
		"ws_6":         {core.SchemeWorkspace6, 5},
		"ws_7":         {core.SchemeWorkspace7, 6},
		"ws_8":         {core.SchemeWorkspace8, 7},
		"ws_9":         {core.SchemeWorkspace9, 8},
		"tab_selected": {core.TabSel, -1},
		"tab_normal":   {core.TabNorm, -1},
	}

	// 1-color schemes: just fg
	oneColorSchemes := map[string]core.Scheme{
		"button_prev":  core.SchemeBtnPrev,
		"button_next":  core.SchemeBtnNext,
		"button_close": core.SchemeBtnClose,
	}

	themeTable.ForEach(func(k, v lua.LValue) {
		key := k.String()

		// 2-color scheme: table {fg, bg}
		if tbl, ok := v.(*lua.LTable); ok {
			info, exists := twoColorSchemes[key]
			if !exists {
				return
			}
			if fg := tbl.RawGetInt(1); fg.Type() == lua.LTString {
				core.SetColor(info.scheme, 0, fg.String())
			}
			if bg := tbl.RawGetInt(2); bg.Type() == lua.LTString {
				core.SetColor(info.scheme, 1, bg.String())
			}
			if info.wsIdx >= 0 {
				core.SetWorkspaceScheme(info.wsIdx, info.scheme)
			}
			return
		}

		// String value
		if str, ok := v.(lua.LString); ok {
			hex := string(str)
			// 1-color scheme (button)
			if scheme, exists := oneColorSchemes[key]; exists {
				core.SetColor(scheme, 0, hex)
				return
			}
			// Widget palette variable
			state.palette[key] = hex
			replacements = append(replacements, "{"+key+"}", hex)
		}
	})

	state.themeRepl = strings.NewReplacer(replacements...)
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
