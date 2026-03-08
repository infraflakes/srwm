package ipc

import (
	"bufio"
	"fmt"
	"net"

	"github.com/nixuris/srwm/internal/config"
	"github.com/nixuris/srwm/internal/core"
)

func handleConnection(conn net.Conn) {
	defer conn.Close()
	scanner := bufio.NewScanner(conn)
	for scanner.Scan() {
		command := scanner.Text()
		switch command {
		case "shutdown":
			fmt.Println("control: received shutdown request")
			core.Quit()
			return
		case "restart":
			fmt.Println("control: received restart request")
			core.Restart()
			return
		case "refresh":
			config.NotifyBarRefresh()
		default:
			fmt.Printf("control: unknown command received: %q\n", command)
		}
	}
}

// Send sends a command to a running srwm instance via the Unix socket.
func Send(socketPath, command string) error {
	conn, err := net.Dial("unix", socketPath)
	if err != nil {
		return fmt.Errorf("could not connect to srwm: %w (is it running?)", err)
	}
	defer conn.Close()

	_, err = fmt.Fprintln(conn, command)
	if err != nil {
		return fmt.Errorf("failed to send command: %w", err)
	}

	return nil
}
