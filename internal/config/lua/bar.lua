--------------------------------------------------------------------------------
-- Status Bar and Tab Bar Setup
--------------------------------------------------------------------------------

-- Bar
srwm.cfg.bar_horizontal_padding(0)
srwm.cfg.bar_vertical_padding(15)

-- Tab
srwm.cfg.tab_vertical_padding(35) -- set to 0 breaks wm
srwm.cfg.tab_in_horizontal_padding(15)
srwm.cfg.tab_out_horizontal_padding(15)

-- Looks
srwm.cfg.showbar(true)
srwm.cfg.topbar(true) -- need reimplementation
srwm.cfg.toptab(true) -- need reimplementation

-- Set fonts
srwm.bar.fonts("JetBrainsMonoNerdFont:size=13")

-- Tag underline
srwm.cfg.tag_underline_padding(5)
srwm.cfg.tag_underline_size(2)
srwm.cfg.tag_underline_offset_from_bar_bottom(0)
srwm.cfg.tag_underline_for_all_tags(false)

-- Systray
srwm.cfg.systray_enable(true)
srwm.cfg.systray_spacing(2)
srwm.cfg.systray_pinning(0) -- value is monitors order, 0 is first monitor

-- Tag preview
srwm.cfg.tag_preview_size(4)
srwm.cfg.tag_preview_enable(false)

-- Set polling interval (seconds)
srwm.bar.interval(1)

