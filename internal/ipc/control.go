package ipc

import (
	"fmt"
	"net"
	"os"
	"path/filepath"
)

// DefaultSocketPath returns the path to the Unix socket used for IPC.
func DefaultSocketPath() string {
	// Try to use XDG_RUNTIME_DIR if available, fallback to /tmp
	runtimeDir := os.Getenv("XDG_RUNTIME_DIR")
	if runtimeDir != "" {
		return filepath.Join(runtimeDir, "srwm.sock")
	}
	return "/tmp/srwm.sock"
}

// Listen starts the IPC server and listens for commands.
// It blocks until the socket can no longer be served.
func Listen(socketPath string) error {
	// Remove existing socket if it exists
	if _, err := os.Stat(socketPath); err == nil {
		if err := os.Remove(socketPath); err != nil {
			return fmt.Errorf("failed to remove existing socket: %w", err)
		}
	}

	ln, err := net.Listen("unix", socketPath)
	if err != nil {
		return fmt.Errorf("failed to listen on socket: %w", err)
	}
	defer ln.Close()
	defer os.Remove(socketPath)

	for {
		conn, err := ln.Accept()
		if err != nil {
			return err
		}
		go handleConnection(conn)
	}
}
