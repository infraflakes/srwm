//! FocusTarget newtype — the SeatHandler focus type for keyboard, pointer, and touch.
//!
//! Required because `PopupGrab` needs `KeyboardFocus: From<PopupKind>`, and we
//! can't impl `From<PopupKind> for WlSurface` (orphan rule). All input-target
//! Keyboard methods route through `X11Surface` when applicable (for ICCCM
//! `SetInputFocus` + `WM_TAKE_FOCUS`); pointer/touch delegate to `WlSurface`.

use std::borrow::Cow;

use smithay::{
    backend::input::KeyState,
    desktop::PopupKind,
    input::{
        Seat,
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, MotionEvent,
            PointerTarget, RelativeMotionEvent,
        },
        touch::{
            DownEvent as TouchDownEvent, MotionEvent as TouchMotionEvent, TouchTarget,
            UpEvent as TouchUpEvent,
        },
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
    xwayland::X11Surface,
};

use crate::state::Srwc;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::{
    input::dnd::{DndFocus, Source},
    utils::{Logical, Point},
    wayland::selection::data_device::WlOfferData,
};
use std::sync::Arc;

// --- FocusTarget ---
// Newtype over WlSurface for use as SeatHandler focus types.
// Required because PopupGrab needs `KeyboardFocus: From<PopupKind>`,
// and we can't impl `From<PopupKind> for WlSurface` (orphan rule).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTarget(pub WlSurface);

impl From<PopupKind> for FocusTarget {
    fn from(popup: PopupKind) -> Self {
        FocusTarget(popup.wl_surface().clone())
    }
}

impl IsAlive for FocusTarget {
    fn alive(&self) -> bool {
        self.0.alive()
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        Some(Cow::Borrowed(&self.0))
    }
}

// Delegate all KeyboardTarget methods to the inner WlSurface using
// fully-qualified syntax to avoid clash with WlSurface::enter() protocol method.
impl KeyboardTarget<Srwc> for FocusTarget {
    fn enter(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Srwc>>::enter(&x11, seat, data, keys, serial);
        } else {
            <WlSurface as KeyboardTarget<Srwc>>::enter(&self.0, seat, data, keys, serial);
        }
    }

    fn leave(&self, seat: &Seat<Srwc>, data: &mut Srwc, serial: Serial) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Srwc>>::leave(&x11, seat, data, serial);
        } else {
            <WlSurface as KeyboardTarget<Srwc>>::leave(&self.0, seat, data, serial);
        }
    }

    fn key(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Srwc>>::key(&x11, seat, data, key, state, serial, time);
        } else {
            <WlSurface as KeyboardTarget<Srwc>>::key(&self.0, seat, data, key, state, serial, time);
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        if let Some(x11) = data.find_x11_surface_by_wl(&self.0) {
            <X11Surface as KeyboardTarget<Srwc>>::modifiers(&x11, seat, data, modifiers, serial);
        } else {
            <WlSurface as KeyboardTarget<Srwc>>::modifiers(&self.0, seat, data, modifiers, serial);
        }
    }
}

impl PointerTarget<Srwc> for FocusTarget {
    fn enter(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &MotionEvent) {
        <WlSurface as PointerTarget<Srwc>>::enter(&self.0, seat, data, event);
    }

    fn motion(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &MotionEvent) {
        <WlSurface as PointerTarget<Srwc>>::motion(&self.0, seat, data, event);
    }

    fn relative_motion(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &RelativeMotionEvent) {
        <WlSurface as PointerTarget<Srwc>>::relative_motion(&self.0, seat, data, event);
    }

    fn button(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &ButtonEvent) {
        <WlSurface as PointerTarget<Srwc>>::button(&self.0, seat, data, event);
    }

    fn axis(&self, seat: &Seat<Srwc>, data: &mut Srwc, frame: AxisFrame) {
        <WlSurface as PointerTarget<Srwc>>::axis(&self.0, seat, data, frame);
    }

    fn frame(&self, seat: &Seat<Srwc>, data: &mut Srwc) {
        <WlSurface as PointerTarget<Srwc>>::frame(&self.0, seat, data);
    }

    fn gesture_swipe_begin(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &GestureSwipeBeginEvent,
    ) {
        <WlSurface as PointerTarget<Srwc>>::gesture_swipe_begin(&self.0, seat, data, event);
    }

    fn gesture_swipe_update(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &GestureSwipeUpdateEvent,
    ) {
        <WlSurface as PointerTarget<Srwc>>::gesture_swipe_update(&self.0, seat, data, event);
    }

    fn gesture_swipe_end(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &GestureSwipeEndEvent) {
        <WlSurface as PointerTarget<Srwc>>::gesture_swipe_end(&self.0, seat, data, event);
    }

    fn gesture_pinch_begin(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &GesturePinchBeginEvent,
    ) {
        <WlSurface as PointerTarget<Srwc>>::gesture_pinch_begin(&self.0, seat, data, event);
    }

    fn gesture_pinch_update(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &GesturePinchUpdateEvent,
    ) {
        <WlSurface as PointerTarget<Srwc>>::gesture_pinch_update(&self.0, seat, data, event);
    }

    fn gesture_pinch_end(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &GesturePinchEndEvent) {
        <WlSurface as PointerTarget<Srwc>>::gesture_pinch_end(&self.0, seat, data, event);
    }

    fn gesture_hold_begin(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &GestureHoldBeginEvent,
    ) {
        <WlSurface as PointerTarget<Srwc>>::gesture_hold_begin(&self.0, seat, data, event);
    }

    fn gesture_hold_end(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &GestureHoldEndEvent) {
        <WlSurface as PointerTarget<Srwc>>::gesture_hold_end(&self.0, seat, data, event);
    }

    fn leave(&self, seat: &Seat<Srwc>, data: &mut Srwc, serial: Serial, time: u32) {
        <WlSurface as PointerTarget<Srwc>>::leave(&self.0, seat, data, serial, time);
    }
}

impl TouchTarget<Srwc> for FocusTarget {
    fn down(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &TouchDownEvent, seq: Serial) {
        <WlSurface as TouchTarget<Srwc>>::down(&self.0, seat, data, event, seq);
    }

    fn up(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &TouchUpEvent, seq: Serial) {
        <WlSurface as TouchTarget<Srwc>>::up(&self.0, seat, data, event, seq);
    }

    fn motion(&self, seat: &Seat<Srwc>, data: &mut Srwc, event: &TouchMotionEvent, seq: Serial) {
        <WlSurface as TouchTarget<Srwc>>::motion(&self.0, seat, data, event, seq);
    }

    fn frame(&self, seat: &Seat<Srwc>, data: &mut Srwc, seq: Serial) {
        <WlSurface as TouchTarget<Srwc>>::frame(&self.0, seat, data, seq);
    }

    fn cancel(&self, seat: &Seat<Srwc>, data: &mut Srwc, seq: Serial) {
        <WlSurface as TouchTarget<Srwc>>::cancel(&self.0, seat, data, seq);
    }

    fn shape(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &smithay::input::touch::ShapeEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Srwc>>::shape(&self.0, seat, data, event, seq);
    }

    fn orientation(
        &self,
        seat: &Seat<Srwc>,
        data: &mut Srwc,
        event: &smithay::input::touch::OrientationEvent,
        seq: Serial,
    ) {
        <WlSurface as TouchTarget<Srwc>>::orientation(&self.0, seat, data, event, seq);
    }
}

impl DndFocus<Srwc> for FocusTarget {
    type OfferData<S>
        = WlOfferData<S>
    where
        S: Source;

    fn enter<S: Source>(
        &self,
        data: &mut Srwc,
        dh: &DisplayHandle,
        source: Arc<S>,
        seat: &Seat<Srwc>,
        location: Point<f64, Logical>,
        serial: &Serial,
    ) -> Option<WlOfferData<S>> {
        <WlSurface as DndFocus<Srwc>>::enter(&self.0, data, dh, source, seat, location, serial)
    }

    fn motion<S: Source>(
        &self,
        data: &mut Srwc,
        offer: Option<&mut WlOfferData<S>>,
        seat: &Seat<Srwc>,
        location: Point<f64, Logical>,
        time: u32,
    ) {
        <WlSurface as DndFocus<Srwc>>::motion(&self.0, data, offer, seat, location, time)
    }

    fn leave<S: Source>(
        &self,
        data: &mut Srwc,
        offer: Option<&mut WlOfferData<S>>,
        seat: &Seat<Srwc>,
    ) {
        <WlSurface as DndFocus<Srwc>>::leave(&self.0, data, offer, seat)
    }

    fn drop<S: Source>(
        &self,
        data: &mut Srwc,
        offer: Option<&mut WlOfferData<S>>,
        seat: &Seat<Srwc>,
    ) {
        <WlSurface as DndFocus<Srwc>>::drop(&self.0, data, offer, seat)
    }
}
