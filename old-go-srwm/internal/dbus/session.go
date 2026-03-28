package dbus

import (
	"bufio"
	"fmt"
	"log"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"time"
)

type Session struct {
	cmd     *exec.Cmd
	address string
}

func Start() (*Session, error) {
	if addr := os.Getenv("DBUS_SESSION_BUS_ADDRESS"); addr != "" {
		return nil, nil // already running
	}

	dbusPath, err := exec.LookPath("dbus-daemon")
	if err != nil {
		log.Printf("dbus-daemon not found, skipping D-Bus session bus")
		return nil, nil
	}

	cmd := exec.Command(dbusPath, "--session", "--print-address", "--nofork", "--nopidfile")
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, fmt.Errorf("dbus: failed to create stdout pipe: %w", err)
	}
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("dbus: failed to start dbus-daemon: %w", err)
	}

	// Read the bus address from the first line of stdout
	scanner := bufio.NewScanner(stdout)
	addrCh := make(chan string, 1)
	go func() {
		if scanner.Scan() {
			addrCh <- strings.TrimSpace(scanner.Text())
		}
	}()

	select {
	case addr := <-addrCh:
		if err := os.Setenv("DBUS_SESSION_BUS_ADDRESS", addr); err != nil {
			log.Printf("dbus: failed to set DBUS_SESSION_BUS_ADDRESS: %v", err)
		}
		log.Printf("D-Bus session bus started: %s", addr)
		return &Session{cmd: cmd, address: addr}, nil
	case <-time.After(5 * time.Second):
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		return nil, fmt.Errorf("dbus: timed out waiting for bus address")
	}
}

func (s *Session) Stop() {
	if s == nil || s.cmd == nil || s.cmd.Process == nil {
		return
	}
	_ = s.cmd.Process.Signal(syscall.SIGTERM)
	_ = s.cmd.Wait()
	log.Printf("D-Bus session bus stopped")
}
