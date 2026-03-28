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
	ClkTabBar    = 0
	ClkTabPrev   = 1
	ClkTabNext   = 2
	ClkTabClose  = 3
	ClkClientWin = 4
	ClkRootWin   = 5
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
