package cmd

import (
	"os"

	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "srwm",
	Short: "srwm is a fully statically linked X11 window manager",
	Long:  `srwm is a fully statically linked X11 window manager powered by Go and C.`,
}

// Execute adds all child commands to the root command and sets flags appropriately.
func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	// Root flags can be defined here
}
