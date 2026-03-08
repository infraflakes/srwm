package main

import (
	"fmt"
	"log"
	"os"

	"github.com/nixuris/srwm/internal/core"
	"github.com/nixuris/srwm/internal/ipc"
)

// Version is set at build time via -ldflags.
var Version = "dev"

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	socketPath := ipc.DefaultSocketPath()

	switch os.Args[1] {
	case "start":
		runWM(socketPath)
	case "shutdown":
		if err := ipc.Send(socketPath, "shutdown"); err != nil {
			log.Fatalf("shutdown failed: %v", err)
		}
	case "restart":
		if err := ipc.Send(socketPath, "restart"); err != nil {
			log.Fatalf("restart failed: %v", err)
		}
	case "version":
		fmt.Printf("srwm %s\n", Version)
	default:
		fmt.Fprintf(os.Stderr, "srwm: unknown command %q\n", os.Args[1])
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Fprintln(os.Stderr, "Usage: srwm <command>")
	fmt.Fprintln(os.Stderr, "")
	fmt.Fprintln(os.Stderr, "Commands:")
	fmt.Fprintln(os.Stderr, "  start      Start the window manager")
	fmt.Fprintln(os.Stderr, "  shutdown   Shut down the running window manager")
	fmt.Fprintln(os.Stderr, "  restart    Soft-restart the running window manager")
	fmt.Fprintln(os.Stderr, "  version    Print version information")
}

func runWM(socketPath string) {
	log.SetPrefix("srwm: ")
	log.SetFlags(0)

	// Start IPC server
	go func() {
		if err := ipc.Listen(socketPath); err != nil {
			log.Printf("IPC server error: %v", err)
		}
	}()

	for {
		if err := core.Init(); err != nil {
			log.Fatalf("init failed: %v", err)
		}

		log.Println("started")
		core.Run() // blocks until quit or restart
		core.Cleanup()

		if !core.ShouldRestart() {
			log.Println("exiting")
			break
		}

		log.Println("restarting...")
		// loop back → re-init
	}
}
