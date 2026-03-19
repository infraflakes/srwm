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
		if L.GetTop() == 1 {
			core.SetBarHorizontalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetBarHorizontalPadding()))
		}
		return 1
	}))

	barTable.RawSetString("padding_vertical", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetBarVerticalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetBarVerticalPadding()))
		}
		return 1
	}))

	barTable.RawSetString("tab_height", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabHeight(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabHeight()))
		}
		return 1
	}))

	barTable.RawSetString("tab_tile_vertical_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabTileVerticalPadding(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabTileVerticalPadding()))
		}
		return 1
	}))

	barTable.RawSetString("tab_tile_outer_padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabTileOuterPaddingHorizontal(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabTileOuterPaddingHorizontal()))
		}
		return 1
	}))

	barTable.RawSetString("tab_tile_inner_padding_horizontal", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTabTileInnerPaddingHorizontal(L.CheckInt(1))
		} else {
			L.Push(lua.LNumber(core.GetTabTileInnerPaddingHorizontal()))
		}
		return 1
	}))

	barTable.RawSetString("show", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetShowbar(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetShowbar()))
		}
		return 1
	}))

	barTable.RawSetString("top", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTopBar(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTopBar()))
		}
		return 1
	}))

	barTable.RawSetString("tab_top", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTopTab(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTopTab()))
		}
		return 1
	}))

	barTable.RawSetString("systray", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetSystrayEnable(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetSystrayEnable()))
		}
		return 1
	}))

	barTable.RawSetString("systray_spacing", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetSystraySpacing(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetSystraySpacing()))
		}
		return 1
	}))

	barTable.RawSetString("systray_pinning", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetSystrayPinning(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetSystrayPinning()))
		}
		return 1
	}))

	barTable.RawSetString("tag_underline_padding", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlinePadding(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetTagUnderlinePadding()))
		}
		return 1
	}))

	barTable.RawSetString("tag_underline_size", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlineSize(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetTagUnderlineSize()))
		}
		return 1
	}))

	barTable.RawSetString("tag_underline_offset", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlineOffsetFromBarBottom(uint(L.CheckInt(1)))
		} else {
			L.Push(lua.LNumber(core.GetTagUnderlineOffsetFromBarBottom()))
		}
		return 1
	}))

	barTable.RawSetString("tag_underline_all_tags", L.NewFunction(func(L *lua.LState) int {
		if L.GetTop() == 1 {
			core.SetTagUnderlineForAllTags(L.CheckBool(1))
		} else {
			L.Push(lua.LBool(core.GetTagUnderlineForAllTags()))
		}
		return 1
	}))

	barTable.RawSetString("theme", L.NewFunction(func(L *lua.LState) int {
		return luaBarTheme(L, state)
	}))

	// Nested tags table
	tagTable := L.NewTable()
	tagTable.RawSetString("highlight_occupied_only", L.NewFunction(func(L *lua.LState) int {
		core.SetTagColorfulOccupiedOnly(L.CheckBool(1))
		return 0
	}))
	barTable.RawSetString("tags", tagTable)

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

	schemeMap := map[string]struct {
		scheme core.Scheme
		tagIdx int // -1 if not a tag scheme
	}{
		"normal":       {core.SchemeNorm, -1},
		"selected":     {core.SchemeSel, -1},
		"title":        {core.SchemeTitle, -1},
		"inactive_tag": {core.SchemeTag, -1},
		"tag_1":        {core.SchemeTag1, 0},
		"tag_2":        {core.SchemeTag2, 1},
		"tag_3":        {core.SchemeTag3, 2},
		"tag_4":        {core.SchemeTag4, 3},
		"tag_5":        {core.SchemeTag5, 4},
		"tag_6":        {core.SchemeTag6, 5},
		"tag_7":        {core.SchemeTag7, 6},
		"tag_8":        {core.SchemeTag8, 7},
		"tag_9":        {core.SchemeTag9, 8},
		"tab_selected": {core.TabSel, -1},
		"tab_normal":   {core.TabNorm, -1},
		"button_prev":  {core.SchemeBtnPrev, -1},
		"button_next":  {core.SchemeBtnNext, -1},
		"button_close": {core.SchemeBtnClose, -1},
	}

	themeTable.ForEach(func(k, v lua.LValue) {
		key := k.String()

		// Nested table → WM core color
		if tbl, ok := v.(*lua.LTable); ok {
			info, exists := schemeMap[key]
			if !exists {
				return
			}
			scheme := info.scheme
			if fg := tbl.RawGetString("fg"); fg.Type() == lua.LTString {
				core.SetColor(scheme, 0, fg.String())
			}
			if bg := tbl.RawGetString("bg"); bg.Type() == lua.LTString {
				core.SetColor(scheme, 1, bg.String())
			}
			if border := tbl.RawGetString("border"); border.Type() == lua.LTString {
				core.SetColor(scheme, 2, border.String())
			}

			// If it's a tag scheme, automatically map the tag to it
			if info.tagIdx >= 0 {
				core.SetTagScheme(info.tagIdx, scheme)
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
