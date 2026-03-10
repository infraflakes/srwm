package cmd

import (
	"context"
	"log"

	"github.com/nixuris/srwm/internal/config"
	"github.com/nixuris/srwm/internal/control"
	"github.com/nixuris/srwm/internal/core"
	"github.com/nixuris/srwm/internal/ipc"
	"github.com/spf13/cobra"
)

var startCmd = &cobra.Command{
	Use:   "start",
	Short: "Start the window manager",
	Run: func(cmd *cobra.Command, args []string) {
		socketPath := ipc.DefaultSocketPath()
		runWM(socketPath)
	},
}

func init() {
	rootCmd.AddCommand(startCmd)
}

func runWM(socketPath string) {
	log.SetPrefix("srwm: ")
	log.SetFlags(0)

	// Wire the control package's spawn-refresh callback to the
	// config package's bar refresh channel.
	control.SetRefreshNotifier(config.NotifyBarRefresh)

	// Start IPC server
	go func() {
		if err := ipc.Listen(socketPath); err != nil {
			log.Printf("IPC server error: %v", err)
		}
	}()

	for {
		if err := core.InitDisplay(); err != nil {
			log.Fatalf("init failed: %v", err)
		}

		// Start Lua config — sets fonts, colors, padding, etc.
		ctx, cancel := context.WithCancel(context.Background())
		config.StartConfig(ctx)

		// Wait for config values to be applied before X11 setup
		config.WaitConfigReady()

		// Now setup() reads the correct config values
		core.InitSetup()

		core.Run() // blocks until `running = 0`

		// Stop Lua before cleaning up C core
		cancel()
		core.Cleanup()

		if !core.ShouldRestart() {
			break // clean exit, not restart
		}
		// loop back -> re-init for hot-reload
	}
}
