package cmd

import (
	"log"

	"github.com/infraflakes/srwm/internal/ipc"
	"github.com/spf13/cobra"
)

var restartCmd = &cobra.Command{
	Use:   "restart",
	Short: "Soft-restart the running window manager",
	Run: func(cmd *cobra.Command, args []string) {
		socketPath := ipc.DefaultSocketPath()
		if err := ipc.Send(socketPath, "restart"); err != nil {
			log.Fatalf("restart failed: %v", err)
		}
	},
}

func init() {
	rootCmd.AddCommand(restartCmd)
}
