package cmd

import (
	"context"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/infraflakes/srwm/internal/config"
	"github.com/infraflakes/srwm/internal/control"
	"github.com/infraflakes/srwm/internal/core"
	"github.com/infraflakes/srwm/internal/ipc"
	"github.com/infraflakes/srwm/internal/xserver"
	"github.com/spf13/cobra"
)

var startCmd = &cobra.Command{
	Use:   "start",
	Short: "Start the window manager",
	Long: `Start the srwm window manager.  
  
If $DISPLAY is not set (e.g., running from a TTY), srwm will automatically  
start an Xorg server. Otherwise, it connects to the existing X display.`,
	Run: func(cmd *cobra.Command, args []string) {
		socketPath := ipc.DefaultSocketPath()

		if os.Getenv("DISPLAY") == "" {
			srv, err := xserver.Start()
			if err != nil {
				log.Fatalf("failed to start X server: %v", err)
			}
			defer srv.Stop()

			sigCh := make(chan os.Signal, 1)
			signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT, syscall.SIGHUP)
			go func() {
				<-sigCh
				srv.Stop()
				os.Exit(0)
			}()
		}

		runWM(socketPath)
	},
}

func init() {
	rootCmd.AddCommand(startCmd)
}

func runWM(socketPath string) {
	log.SetPrefix("srwm: ")
	log.SetFlags(0)

	control.SetRefreshNotifier(config.NotifyBarRefresh)

	go func() {
		if err := ipc.Listen(socketPath); err != nil {
			log.Printf("IPC server error: %v", err)
		}
	}()

	for {
		if err := core.InitDisplay(); err != nil {
			log.Fatalf("init failed: %v", err)
		}

		ctx, cancel := context.WithCancel(context.Background())
		config.StartConfig(ctx)
		config.WaitConfigReady()
		core.InitSetup()
		core.Run()
		cancel()
		core.Cleanup()

		if !core.ShouldRestart() {
			break
		}
		core.ClearKeybindings()
	}
}
