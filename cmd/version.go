package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// Version is set at build time via -ldflags in Makefile.
var Version = "dev"

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print version information",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("srwm %s\n", Version)
	},
}

func init() {
	rootCmd.AddCommand(versionCmd)
}
