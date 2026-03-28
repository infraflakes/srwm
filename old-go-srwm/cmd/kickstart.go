package cmd

import (
	"fmt"
	"log"

	"github.com/infraflakes/srwm/internal/config"
	"github.com/spf13/cobra"
)

var kickstartCmd = &cobra.Command{
	Use:   "kickstart",
	Short: "Deploy default Lua configuration files",
	Long: `Deploy the default srwmrc.lua and bar.lua to ~/.config/srwm/

This command will fail if a config file already exists.
Backup your existing config before running this command.`,
	Run: func(cmd *cobra.Command, args []string) {
		runKickstart()
	},
}

func init() {
	rootCmd.AddCommand(kickstartCmd)
}

func runKickstart() {
	log.SetPrefix("srwm: ")
	log.SetFlags(0)

	if err := config.DeployKickstart(); err != nil {
		log.Fatal(err)
	}

	fmt.Println("kickstart complete!")
}
