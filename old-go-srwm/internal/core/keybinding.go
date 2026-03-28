package core

/*
#include "bridge.h"
*/
import "C"

import (
	"log"
	"sync"
)

var (
	keyCallbacks = make(map[int]func())
	keyIDCounter = 0
	keyMutex     sync.RWMutex
)

//export srwm_handle_key
func srwm_handle_key(id C.int) {
	keyMutex.RLock()
	cb, ok := keyCallbacks[int(id)]
	keyMutex.RUnlock()

	if ok && cb != nil {
		// Execute the callback synchronously so Xlib calls via C stay
		// on the main X11 event loop thread. Executing in a goroutine
		// causes severe GC/theme corruption.
		defer func() {
			if r := recover(); r != nil {
				log.Printf("srwm: recovered from panic in key callback %d: %v", id, r)
			}
		}()
		cb()
	} else {
		log.Printf("srwm: no callback found for key id %d", id)
	}
}

// AddKeybinding registers a new dynamic keybinding with C and stores the Go callback.
func AddKeybinding(mod uint, keysym uint, cb func()) {
	keyMutex.Lock()
	id := keyIDCounter
	keyIDCounter++
	keyCallbacks[id] = cb
	keyMutex.Unlock()

	C.srwm_add_keybinding(C.uint(mod), C.KeySym(keysym), C.int(id))
}

// ClearKeybindings removes all dynamic keybindings from memory.
func ClearKeybindings() {
	keyMutex.Lock()
	keyCallbacks = make(map[int]func())
	keyIDCounter = 0
	keyMutex.Unlock()

	C.srwm_clear_keybindings()
}

// GrabKeys tells the X server to grab the updated keybindings.
func GrabKeys() {
	C.srwm_grabkeys()
}
