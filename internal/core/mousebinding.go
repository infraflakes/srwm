package core

/*
#include "bridge.h"
*/
import "C"

import (
	"log"
	"sync"
)

const (
	ClkWsBar      = 0
	ClkTabBar     = 1
	ClkTabPrev    = 2
	ClkTabNext    = 3
	ClkTabClose   = 4
	ClkStatusText = 5
	ClkWinTitle   = 6
	ClkClientWin  = 7
	ClkRootWin    = 8
)

var (
	mouseCallbacks = make(map[int]func())
	mouseIDCounter = 0
	mouseMutex     sync.RWMutex
)

//export srwm_handle_mouse
func srwm_handle_mouse(id C.int) {
	mouseMutex.RLock()
	cb, ok := mouseCallbacks[int(id)]
	mouseMutex.RUnlock()

	if ok && cb != nil {
		defer func() {
			if r := recover(); r != nil {
				log.Printf("srwm: recovered from panic in mouse callback %d: %v", id, r)
			}
		}()
		cb()
	} else {
		log.Printf("srwm: no callback found for mouse id %d", id)
	}
}

func AddMouseBinding(click uint, mod uint, button uint, cb func()) {
	mouseMutex.Lock()
	id := mouseIDCounter
	mouseIDCounter++
	mouseCallbacks[id] = cb
	mouseMutex.Unlock()

	C.srwm_add_mousebinding(C.uint(click), C.uint(mod), C.uint(button), C.int(id))
}

func ClearMouseBindings() {
	mouseMutex.Lock()
	mouseCallbacks = make(map[int]func())
	mouseIDCounter = 0
	mouseMutex.Unlock()

	C.srwm_clear_mousebindings()
}
