-- Pan canvas with arrow keys (0=left, 1=right, 2=up, 3=down)
srwm.key.bind("Mod4", "Left", function()
	srwm.canvas.move(0)
end)
srwm.key.bind("Mod4", "Right", function()
	srwm.canvas.move(1)
end)
srwm.key.bind("Mod4", "Up", function()
	srwm.canvas.move(2)
end)
srwm.key.bind("Mod4", "Down", function()
	srwm.canvas.move(3)
end)

-- Reset canvas to origin
srwm.key.bind("Mod4", "h", function()
	srwm.canvas.home()
end)

-- Center canvas on focused window
srwm.key.bind("Mod4", "g", function()
	srwm.canvas.center_window()
end)

-- Zoom in/out canvas
srwm.key.bind("Mod4", "equal", function()
	srwm.canvas.zoom(1)
end)
srwm.key.bind("Mod4", "minus", function()
	srwm.canvas.zoom(-1)
end)
