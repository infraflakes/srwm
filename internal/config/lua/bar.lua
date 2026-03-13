--------------------------------------------------------------------------------
-- Status Bar and Tab Bar Setup
--------------------------------------------------------------------------------

-- Bar
srwm.bar.padding_horizontal(0)
srwm.bar.padding_vertical(15)

-- Tab
srwm.bar.tab_padding_vertical(35) -- set to 0 breaks wm
srwm.bar.tab_padding_inner_horizontal(15)
srwm.bar.tab_padding_outer_horizontal(15)

-- Looks
srwm.bar.show(true)
srwm.bar.top(true)
-- srwm.bar.tab_show(true) -- incoming feature
srwm.bar.tab_top(true)

-- Set fonts
srwm.bar.fonts("JetBrainsMonoNerdFont:size=13")
-- srwm.bar.fonts("Terminess Nerd Font:size=14")

-- Tag underline
srwm.bar.tag_underline_padding(5)
srwm.bar.tag_underline_size(2)
srwm.bar.tag_underline_offset(0)
srwm.bar.tag_underline_all_tags(false)

-- Systray
srwm.bar.systray(true)
srwm.bar.systray_spacing(2)
srwm.bar.systray_pinning(0) -- value is monitors order, 0 is first monitor

-- Tag preview
srwm.bar.tag_preview_size(4)
srwm.bar.tag_preview(true)

-- Set polling interval (seconds)
srwm.bar.interval(1)
