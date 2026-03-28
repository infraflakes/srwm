package cmd

import (
	"log"

	"github.com/infraflakes/srwm/internal/ipc"
	"github.com/spf13/cobra"
)

var barCmd = &cobra.Command{
	Use:   "bar",
	Short: "Status bar management commands",
}

var refreshCmd = &cobra.Command{
	Use:   "refresh",
	Short: "Trigger an immediate status bar refresh",
	Run: func(cmd *cobra.Command, args []string) {
		socketPath := ipc.DefaultSocketPath()
		if err := ipc.Send(socketPath, "refresh"); err != nil {
			log.Fatalf("refresh failed: %v", err)
		}
	},
}

func init() {
	rootCmd.AddCommand(barCmd)
	barCmd.AddCommand(refreshCmd)
}
