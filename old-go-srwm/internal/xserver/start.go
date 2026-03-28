package xserver

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

// Server represents a managed Xorg server process.
type Server struct {
	cmd       *exec.Cmd
	display   string
	xauth     string
	sttyState string
}

// Start launches an Xorg server on the current TTY.
// It sets up xauth, starts Xorg, waits for it to signal readiness via SIGUSR1,
// and sets the DISPLAY environment variable.
//
// Returns a Server that must be Stop()'d on exit.
func Start() (*Server, error) {
	// 1. Detect TTY number
	ttyPath, err := os.Readlink("/proc/self/fd/0")
	if err != nil {
		return nil, fmt.Errorf("cannot determine TTY: %w", err)
	}
	// ttyPath is like "/dev/tty2"
	ttyNum := strings.TrimPrefix(ttyPath, "/dev/tty")
	if ttyNum == ttyPath {
		return nil, fmt.Errorf("stdin is not a TTY: %s", ttyPath)
	}

	display := ":" + ttyNum

	// 2. Save terminal state
	sttyState := ""
	if out, err := exec.Command("stty", "-g").Output(); err == nil {
		sttyState = strings.TrimSpace(string(out))
	}

	// 3. Set up xauth
	dataDir := os.Getenv("XDG_DATA_HOME")
	if dataDir == "" {
		home, _ := os.UserHomeDir()
		dataDir = filepath.Join(home, ".local", "share")
	}
	sxDataDir := filepath.Join(dataDir, "sx")
	_ = os.MkdirAll(sxDataDir, 0755)

	xauthPath := os.Getenv("XAUTHORITY")
	if xauthPath == "" {
		xauthPath = filepath.Join(sxDataDir, "xauthority")
	}

	// Touch the xauthority file
	f, err := os.OpenFile(xauthPath, os.O_CREATE|os.O_WRONLY, 0600)
	if err != nil {
		return nil, fmt.Errorf("cannot create xauthority file: %w", err)
	}
	if err := f.Close(); err != nil {
		return nil, fmt.Errorf("cannot close xauthority file: %w", err)
	}

	_ = os.Setenv("XAUTHORITY", xauthPath)

	// Generate random cookie
	cookie := make([]byte, 16)
	if _, err := rand.Read(cookie); err != nil {
		return nil, fmt.Errorf("cannot generate auth cookie: %w", err)
	}
	cookieHex := hex.EncodeToString(cookie)

	// Add xauth entry
	addCmd := exec.Command("xauth", "add", display, "MIT-MAGIC-COOKIE-1", cookieHex)
	if out, err := addCmd.CombinedOutput(); err != nil {
		return nil, fmt.Errorf("xauth add failed: %w (%s)", err, string(out))
	}

	// 4. Set up SIGUSR1 handler for Xorg readiness
	usr1Ch := make(chan os.Signal, 1)
	signal.Notify(usr1Ch, syscall.SIGUSR1)
	defer signal.Stop(usr1Ch)

	// 5. Start Xorg
	// The "trap '' USR1" sets SIG_IGN for USR1 which Xorg inherits.
	// When Xorg sees inherited SIG_IGN for USR1, it sends USR1 to parent when ready.
	xorgCmd := exec.Command("sh", "-c",
		fmt.Sprintf("trap '' USR1 && exec Xorg %s vt%s -keeptty -noreset -auth %q",
			display, ttyNum, xauthPath))
	xorgCmd.Stdin = os.Stdin
	xorgCmd.Stdout = os.Stdout
	xorgCmd.Stderr = os.Stderr

	if err := xorgCmd.Start(); err != nil {
		// Clean up xauth on failure
		_ = exec.Command("xauth", "remove", display).Run()
		return nil, fmt.Errorf("failed to start Xorg: %w", err)
	}

	// 6. Wait for Xorg readiness (SIGUSR1) with timeout
	select {
	case <-usr1Ch:
		// Xorg is ready
	case <-time.After(10 * time.Second):
		_ = xorgCmd.Process.Kill()
		_ = xorgCmd.Wait()
		_ = exec.Command("xauth", "remove", display).Run()
		return nil, fmt.Errorf("timed out waiting for Xorg to start")
	}

	// 7. Set DISPLAY
	_ = os.Setenv("DISPLAY", display)

	log.Printf("X server started on %s (vt%s)", display, ttyNum)

	return &Server{
		cmd:       xorgCmd,
		display:   display,
		xauth:     xauthPath,
		sttyState: sttyState,
	}, nil
}

// Stop kills the Xorg server, removes the xauth entry, and restores terminal state.
func (s *Server) Stop() {
	if s.cmd != nil && s.cmd.Process != nil {
		_ = s.cmd.Process.Signal(syscall.SIGTERM)
		_ = s.cmd.Wait()
	}

	// Remove xauth entry
	_ = exec.Command("xauth", "remove", s.display).Run()

	// Restore terminal state
	if s.sttyState != "" {
		if err := exec.Command("stty", s.sttyState).Run(); err != nil {
			_ = exec.Command("stty", "sane").Run()
		}
	}

	log.Printf("X server stopped")
}
