pub mod pw_utils;

use smithay::utils::Point;
use std::collections::HashSet;
use std::mem;
use std::time::Duration;

use anyhow::Context as _;
use smithay::backend::allocator::format::FormatSet;
use smithay::backend::allocator::gbm::GbmDevice;
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::output::Output;
use smithay::reexports::wayland_server::Resource;
use smithay::utils::{Physical, Scale, Size};
use smithay::wayland::seat::WaylandFocus;

use crate::dbus::mutter_screen_cast::{self, CastSessionId, ScreenCastToSrwc, StreamTargetId};
use crate::render::OutputRenderElements;
use crate::state::Srwc;

use pw_utils::{Cast, CastSizeChange, CastTarget, PipeWire, PwToSrwc};

pub struct Screencasting {
    pub casts: Vec<Cast>,
    pub pw_to_srwc: smithay::reexports::calloop::channel::Sender<PwToSrwc>,
    // Drop PipeWire last, and specifically after casts, to prevent a double-free.
    pub pipewire: Option<PipeWire>,
}

impl Screencasting {
    pub fn new(event_loop: &smithay::reexports::calloop::LoopHandle<'static, Srwc>) -> Self {
        let pw_to_srwc = {
            let (pw_to_srwc, from_pipewire) = smithay::reexports::calloop::channel::channel();
            event_loop
                .insert_source(from_pipewire, move |event, _, state| match event {
                    smithay::reexports::calloop::channel::Event::Msg(msg) => state.on_pw_msg(msg),
                    smithay::reexports::calloop::channel::Event::Closed => (),
                })
                .unwrap();
            pw_to_srwc
        };

        Self {
            casts: vec![],
            pw_to_srwc,
            pipewire: None,
        }
    }
}

impl Srwc {
    fn prepare_pw_cast(&mut self) -> anyhow::Result<(GbmDevice<DrmDeviceFd>, FormatSet)> {
        let gbm = self
            .screencast
            .gbm_device
            .clone()
            .context("no GBM device available")?;

        let casting = self.screencast.screencasting.as_mut().unwrap();

        // Ensure PipeWire is initialized.
        if casting.pipewire.is_none() {
            let pw = PipeWire::new(self.loop_handle.clone(), casting.pw_to_srwc.clone())
                .context("error initializing PipeWire")?;
            casting.pipewire = Some(pw);
        }

        let render_formats = self
            .backend
            .as_mut()
            .map(|b| b.renderer().egl_context().dmabuf_render_formats().clone())
            .unwrap_or_default();

        Ok((gbm, render_formats))
    }

    pub fn on_pw_msg(&mut self, msg: PwToSrwc) {
        match msg {
            PwToSrwc::StopCast { session_id } => self.stop_cast(session_id),
            PwToSrwc::Redraw { stream_id } => {
                // Request a redraw for the output associated with this cast.
                let casting = self.screencast.screencasting.as_ref();
                if let Some(casting) = casting
                    && let Some(cast) = casting.casts.iter().find(|c| c.stream_id == stream_id)
                    && cast.is_active()
                {
                    // Trigger redraws on all active crtcs
                    self.drm
                        .redraws_needed
                        .extend(self.drm.active_crtcs.iter().copied());
                }
            }
            PwToSrwc::FatalError => {
                tracing::warn!("stopping PipeWire due to fatal error");
                if let Some(ref mut casting) = self.screencast.screencasting
                    && let Some(pw) = casting.pipewire.take()
                {
                    let mut ids = HashSet::new();
                    for cast in &casting.casts {
                        ids.insert(cast.session_id);
                    }
                    for id in ids {
                        self.stop_cast(id);
                    }
                    self.loop_handle.remove(pw.token);
                }
            }
        }
    }

    pub fn on_screen_cast_msg(&mut self, msg: ScreenCastToSrwc) {
        match msg {
            ScreenCastToSrwc::StartCast {
                session_id,
                stream_id,
                target,
                cursor_mode,
                signal_ctx,
            } => {
                let (target, size, refresh, alpha) = match target {
                    StreamTargetId::Output { name } => {
                        let output = self.space.outputs().find(|out| out.name() == name);
                        let Some(output) = output else {
                            tracing::warn!(
                                "error starting screencast: requested output is missing"
                            );
                            self.stop_cast(session_id);
                            return;
                        };

                        let (size, refresh) = cast_params_for_output(output);
                        (
                            CastTarget::Output {
                                name: output.name(),
                            },
                            size,
                            refresh,
                            false,
                        )
                    }
                    StreamTargetId::Window { id } => {
                        // Find the window by its Introspect index
                        let window = self.space.elements().find(|w| {
                            w.wl_surface()
                                .map(|s| u64::from(s.id().protocol_id()) == id)
                                .unwrap_or(false)
                        });
                        let Some(window) = window else {
                            tracing::warn!("error starting screencast: window id {id} not found");
                            self.stop_cast(session_id);
                            return;
                        };
                        let geom = window.geometry();
                        let output_scale = self
                            .focused_output
                            .as_ref()
                            .and_then(|o| {
                                o.current_mode()
                                    .map(|_| o.current_scale().fractional_scale())
                            })
                            .unwrap_or(1.0);
                        let scale = smithay::utils::Scale::from(output_scale);
                        let size = geom.size.to_physical_precise_round(scale);
                        let refresh = self
                            .focused_output
                            .as_ref()
                            .and_then(|o| o.current_mode())
                            .map(|m| m.refresh as u32)
                            .unwrap_or(60_000);
                        (
                            CastTarget::Window { id },
                            size,
                            refresh,
                            true, // alpha for window casts
                        )
                    }
                };

                let (gbm, render_formats) = match self.prepare_pw_cast() {
                    Ok(x) => x,
                    Err(err) => {
                        tracing::warn!("error starting screencast: {err:?}");
                        self.stop_cast(session_id);
                        return;
                    }
                };
                let pw = self
                    .screencast
                    .screencasting
                    .as_ref()
                    .unwrap()
                    .pipewire
                    .as_ref()
                    .unwrap();

                let res = pw.start_cast(
                    gbm,
                    render_formats,
                    session_id,
                    stream_id,
                    target,
                    size,
                    refresh,
                    alpha,
                    cursor_mode,
                    signal_ctx,
                );
                match res {
                    Ok(cast) => {
                        self.screencast
                            .screencasting
                            .as_mut()
                            .unwrap()
                            .casts
                            .push(cast);
                    }
                    Err(err) => {
                        tracing::warn!("error starting screencast: {err:?}");
                        self.stop_cast(session_id);
                    }
                }
            }
            ScreenCastToSrwc::StopCast { session_id } => self.stop_cast(session_id),
        }
    }

    pub fn render_for_screen_cast(
        &mut self,
        renderer: &mut GlesRenderer,
        output: &Output,
        elements: &[OutputRenderElements],
        target_presentation_time: Duration,
    ) {
        let mode = output.current_mode().unwrap();
        let transform = output.current_transform();
        let size = transform.transform_size(mode.size);
        let scale = Scale::from(output.current_scale().fractional_scale());

        let mut casts_to_stop = vec![];

        let casting = self.screencast.screencasting.as_mut().unwrap();
        let mut casts = mem::take(&mut casting.casts);
        for cast in &mut casts {
            if !cast.is_active() {
                continue;
            }

            if !cast.target.matches_output(output) {
                continue;
            }

            match cast.ensure_size(size) {
                Ok(CastSizeChange::Ready) => (),
                Ok(CastSizeChange::Pending) => continue,
                Err(err) => {
                    tracing::warn!("error updating stream size, stopping screencast: {err:?}");
                    casts_to_stop.push(cast.session_id);
                    continue;
                }
            }

            if cast.check_time_and_schedule(output, target_presentation_time) {
                continue;
            }

            // Cursor is already embedded in the composed elements, so no separate
            // cursor metadata is needed. Pass None for cursor_data.
            if cast.dequeue_buffer_and_render(renderer, elements, &None, size, scale) {
                cast.last_frame_time = target_presentation_time;
            }
        }
        self.screencast.screencasting.as_mut().unwrap().casts = casts;

        for id in casts_to_stop {
            self.stop_cast(id);
        }
    }

    pub fn render_windows_for_screen_cast(
        &mut self,
        renderer: &mut GlesRenderer,
        output: &Output,
        target_presentation_time: Duration,
    ) {
        let scale = Scale::from(output.current_scale().fractional_scale());
        let mut casts_to_stop = vec![];

        let casting = self.screencast.screencasting.as_mut().unwrap();
        let mut casts = mem::take(&mut casting.casts);
        for cast in &mut casts {
            if !cast.is_active() {
                continue;
            }

            let CastTarget::Window { id } = cast.target else {
                continue;
            };

            // Find the window by Introspect index (same order as space.elements().enumerate())
            let window = self
                .space
                .elements()
                .find(|w| {
                    w.wl_surface()
                        .map(|s| u64::from(s.id().protocol_id()) == id)
                        .unwrap_or(false)
                })
                .cloned();
            let Some(window) = window else {
                continue;
            };

            let geom = window.geometry();
            let size = geom.size.to_physical_precise_round(scale);

            match cast.ensure_size(size) {
                Ok(CastSizeChange::Ready) => (),
                Ok(CastSizeChange::Pending) => continue,
                Err(err) => {
                    tracing::warn!("error updating stream size, stopping screencast: {err:?}");
                    casts_to_stop.push(cast.session_id);
                    continue;
                }
            }

            if cast.check_time_and_schedule(output, target_presentation_time) {
                continue;
            }

            // Render the window's surface tree at (0,0) with zoom=1.0
            use smithay::backend::renderer::element::AsRenderElements;
            use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
            use smithay::backend::renderer::element::utils::RescaleRenderElement;
            use smithay::wayland::seat::WaylandFocus;

            let mut elements: Vec<OutputRenderElements> = Vec::new();
            if let Some(_wl_surface) = window.wl_surface() {
                let surface_elements = window
                    .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                        renderer,
                        // Offset by -geometry.loc so the window content starts at (0,0)
                        Point::from((-geom.loc.x, -geom.loc.y)).to_physical_precise_round(scale),
                        scale,
                        1.0, // full opacity
                    );
                for elem in surface_elements {
                    elements.push(OutputRenderElements::Window(
                        RescaleRenderElement::from_element(
                            elem,
                            Point::<i32, Physical>::from((0, 0)),
                            1.0, // no zoom for window casts
                        ),
                    ));
                }
            }

            if cast.dequeue_buffer_and_render(renderer, &elements, &None, size, scale) {
                cast.last_frame_time = target_presentation_time;
            }
        }
        self.screencast.screencasting.as_mut().unwrap().casts = casts;

        for id in casts_to_stop {
            self.stop_cast(id);
        }
    }

    pub fn stop_cast(&mut self, session_id: CastSessionId) {
        let Some(casting) = self.screencast.screencasting.as_mut() else {
            return;
        };

        for i in (0..casting.casts.len()).rev() {
            let cast = &casting.casts[i];
            if cast.session_id != session_id {
                continue;
            }

            let cast = casting.casts.swap_remove(i);
            if let Err(err) = cast.stream.disconnect() {
                tracing::warn!("error disconnecting stream: {err:?}");
            }
        }

        // Also stop the D-Bus session
        if let Some(ref conn) = self.screencast.conn_screen_cast {
            let server = conn.object_server();
            let path = format!("/org/gnome/Mutter/ScreenCast/Session/u{}", session_id.get());
            if let Ok(iface) = server.interface::<_, mutter_screen_cast::Session>(path) {
                async_io::block_on(async move {
                    iface
                        .get()
                        .stop(server.inner(), iface.signal_emitter().clone())
                        .await
                });
            }
        }
    }
}

fn cast_params_for_output(output: &Output) -> (Size<i32, Physical>, u32) {
    let mode = output.current_mode().unwrap();
    let transform = output.current_transform();
    let size = transform.transform_size(mode.size);
    let refresh = mode.refresh as u32;
    (size, refresh)
}
