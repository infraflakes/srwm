package cmd

import (
	"log"

	"github.com/nixuris/srwm/internal/ipc"
	"github.com/spf13/cobra"
)

var shutdownCmd = &cobra.Command{
	Use:   "shutdown",
	Short: "Shut down the running window manager",
	Run: func(cmd *cobra.Command, args []string) {
		socketPath := ipc.DefaultSocketPath()
		if err := ipc.Send(socketPath, "shutdown"); err != nil {
			log.Fatalf("shutdown failed: %v", err)
		}
	},
}

func init() {
	rootCmd.AddCommand(shutdownCmd)
}
