use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use zbus::fdo::RequestNameFlags;
use zbus::object_server::{InterfaceRef, SignalEmitter};
use zbus::zvariant::{DeserializeDict, OwnedObjectPath, SerializeDict, Type, Value};
use zbus::{ObjectServer, fdo, interface};

use super::Start;

// ---------------------------------------------------------------------------
// ID types (thread-safe monotonic counters)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastSessionId(u64);

impl CastSessionId {
    pub fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    pub fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CastSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastStreamId(u64);

impl CastStreamId {
    pub fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    pub fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CastStreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Output info for the D-Bus interface
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub type IpcOutputMap = Arc<Mutex<HashMap<String, OutputInfo>>>;

// ---------------------------------------------------------------------------
// Cursor mode
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize, Type, Clone, Copy, PartialEq, Eq)]
pub enum CursorMode {
    #[default]
    Hidden = 0,
    Embedded = 1,
    Metadata = 2,
}

// ---------------------------------------------------------------------------
// D-Bus property structs
// ---------------------------------------------------------------------------

#[derive(Debug, DeserializeDict, Type)]
#[zvariant(signature = "dict")]
struct RecordMonitorProperties {
    #[zvariant(rename = "cursor-mode")]
    cursor_mode: Option<CursorMode>,
    #[zvariant(rename = "is-recording")]
    _is_recording: Option<bool>,
}

#[derive(Debug, DeserializeDict, Type)]
#[zvariant(signature = "dict")]
struct RecordWindowProperties {
    #[zvariant(rename = "window-id")]
    window_id: u64,
    #[zvariant(rename = "cursor-mode")]
    cursor_mode: Option<CursorMode>,
    #[zvariant(rename = "is-recording")]
    _is_recording: Option<bool>,
}

#[derive(Debug, SerializeDict, Type, Value)]
#[zvariant(signature = "dict")]
struct StreamParameters {
    position: (i32, i32),
    size: (i32, i32),
}

// ---------------------------------------------------------------------------
// Messages from D-Bus to compositor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum StreamTargetId {
    Output { name: String },
    Window { id: u64 },
}

pub enum ScreenCastToSrwm {
    StartCast {
        session_id: CastSessionId,
        stream_id: CastStreamId,
        target: StreamTargetId,
        cursor_mode: CursorMode,
        signal_ctx: SignalEmitter<'static>,
    },
    StopCast {
        session_id: CastSessionId,
    },
}

// ---------------------------------------------------------------------------
// ScreenCast — main D-Bus object
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ScreenCast {
    ipc_outputs: IpcOutputMap,
    to_srwm: smithay::reexports::calloop::channel::Sender<ScreenCastToSrwm>,
    #[allow(clippy::type_complexity)]
    sessions: Arc<Mutex<Vec<(Session, InterfaceRef<Session>)>>>,
}

#[interface(name = "org.gnome.Mutter.ScreenCast")]
impl ScreenCast {
    async fn create_session(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        properties: HashMap<&str, Value<'_>>,
    ) -> fdo::Result<OwnedObjectPath> {
        if properties.contains_key("remote-desktop-session-id") {
            return Err(fdo::Error::Failed(
                "there are no remote desktop sessions".to_owned(),
            ));
        }

        let session_id = CastSessionId::next();
        let path = format!("/org/gnome/Mutter/ScreenCast/Session/u{}", session_id.get());
        let path = OwnedObjectPath::try_from(path).unwrap();

        let session = Session::new(session_id, self.ipc_outputs.clone(), self.to_srwm.clone());
        match server.at(&path, session.clone()).await {
            Ok(true) => {
                let iface = server.interface(&path).await.unwrap();
                self.sessions.lock().unwrap().push((session, iface));
            }
            Ok(false) => return Err(fdo::Error::Failed("session path already exists".to_owned())),
            Err(err) => {
                return Err(fdo::Error::Failed(format!(
                    "error creating session object: {err:?}"
                )));
            }
        }

        Ok(path)
    }

    #[zbus(property)]
    async fn version(&self) -> i32 {
        4
    }
}

// ---------------------------------------------------------------------------
// Session — per-session D-Bus object
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Session {
    id: CastSessionId,
    ipc_outputs: IpcOutputMap,
    to_srwm: smithay::reexports::calloop::channel::Sender<ScreenCastToSrwm>,
    #[allow(clippy::type_complexity)]
    streams: Arc<Mutex<Vec<(Stream, InterfaceRef<Stream>)>>>,
    stopped: Arc<AtomicBool>,
}

#[interface(name = "org.gnome.Mutter.ScreenCast.Session")]
impl Session {
    async fn start(&self) {
        tracing::debug!("ScreenCast session start");
        for (stream, iface) in &*self.streams.lock().unwrap() {
            stream.start(iface.signal_emitter().clone());
        }
    }

    pub async fn stop(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) {
        tracing::debug!("ScreenCast session stop");

        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }

        Session::closed(&ctxt).await.unwrap();

        if let Err(err) = self.to_srwm.send(ScreenCastToSrwm::StopCast {
            session_id: self.id,
        }) {
            tracing::warn!("error sending StopCast to srwm: {err:?}");
        }

        let streams = mem::take(&mut *self.streams.lock().unwrap());
        for (_, iface) in streams.iter() {
            server
                .remove::<Stream, _>(iface.signal_emitter().path())
                .await
                .unwrap();
        }

        server.remove::<Session, _>(ctxt.path()).await.unwrap();
    }

    async fn record_monitor(
        &mut self,
        #[zbus(object_server)] server: &ObjectServer,
        connector: &str,
        properties: RecordMonitorProperties,
    ) -> fdo::Result<OwnedObjectPath> {
        tracing::debug!(connector, ?properties, "record_monitor");

        let output = {
            let ipc_outputs = self.ipc_outputs.lock().unwrap();
            ipc_outputs.get(connector).cloned()
        };
        let Some(output) = output else {
            return Err(fdo::Error::Failed("no such monitor".to_owned()));
        };

        let stream_id = CastStreamId::next();
        let path = format!("/org/gnome/Mutter/ScreenCast/Stream/u{}", stream_id.get());
        let path = OwnedObjectPath::try_from(path).unwrap();

        let cursor_mode = properties.cursor_mode.unwrap_or_default();

        let target = StreamTarget::Output(output);
        let stream = Stream::new(
            stream_id,
            self.id,
            target,
            cursor_mode,
            self.to_srwm.clone(),
        );
        match server.at(&path, stream.clone()).await {
            Ok(true) => {
                let iface = server.interface(&path).await.unwrap();
                self.streams.lock().unwrap().push((stream, iface));
            }
            Ok(false) => return Err(fdo::Error::Failed("stream path already exists".to_owned())),
            Err(err) => {
                return Err(fdo::Error::Failed(format!(
                    "error creating stream object: {err:?}"
                )));
            }
        }

        Ok(path)
    }

    async fn record_window(
        &mut self,
        #[zbus(object_server)] server: &ObjectServer,
        properties: RecordWindowProperties,
    ) -> fdo::Result<OwnedObjectPath> {
        tracing::debug!(?properties, "record_window");

        let stream_id = CastStreamId::next();
        let path = format!("/org/gnome/Mutter/ScreenCast/Stream/u{}", stream_id.get());
        let path = OwnedObjectPath::try_from(path).unwrap();

        let cursor_mode = properties.cursor_mode.unwrap_or_default();

        let target = StreamTarget::Window {
            id: properties.window_id,
        };
        let stream = Stream::new(
            stream_id,
            self.id,
            target,
            cursor_mode,
            self.to_srwm.clone(),
        );
        match server.at(&path, stream.clone()).await {
            Ok(true) => {
                let iface = server.interface(&path).await.unwrap();
                self.streams.lock().unwrap().push((stream, iface));
            }
            Ok(false) => return Err(fdo::Error::Failed("stream path already exists".to_owned())),
            Err(err) => {
                return Err(fdo::Error::Failed(format!(
                    "error creating stream object: {err:?}"
                )));
            }
        }

        Ok(path)
    }

    #[zbus(signal)]
    async fn closed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}

// ---------------------------------------------------------------------------
// Stream — per-stream D-Bus object
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Stream {
    id: CastStreamId,
    session_id: CastSessionId,
    target: StreamTarget,
    cursor_mode: CursorMode,
    was_started: Arc<AtomicBool>,
    to_srwm: smithay::reexports::calloop::channel::Sender<ScreenCastToSrwm>,
}

#[derive(Clone)]
enum StreamTarget {
    Output(OutputInfo),
    Window { id: u64 },
}

#[interface(name = "org.gnome.Mutter.ScreenCast.Stream")]
impl Stream {
    #[zbus(signal)]
    pub async fn pipe_wire_stream_added(ctxt: &SignalEmitter<'_>, node_id: u32)
    -> zbus::Result<()>;

    #[zbus(property)]
    async fn parameters(&self) -> StreamParameters {
        match &self.target {
            StreamTarget::Output(output) => StreamParameters {
                position: (output.x, output.y),
                size: (output.width as i32, output.height as i32),
            },
            StreamTarget::Window { .. } => StreamParameters {
                position: (0, 0),
                size: (1, 1),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Constructors & helpers
// ---------------------------------------------------------------------------

impl ScreenCast {
    pub fn new(
        ipc_outputs: IpcOutputMap,
        to_srwm: smithay::reexports::calloop::channel::Sender<ScreenCastToSrwm>,
    ) -> Self {
        Self {
            ipc_outputs,
            to_srwm,
            sessions: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl Start for ScreenCast {
    fn start(self) -> anyhow::Result<zbus::blocking::Connection> {
        let conn = zbus::blocking::Connection::session()?;
        let flags = RequestNameFlags::AllowReplacement
            | RequestNameFlags::ReplaceExisting
            | RequestNameFlags::DoNotQueue;

        conn.object_server()
            .at("/org/gnome/Mutter/ScreenCast", self)?;
        conn.request_name_with_flags("org.gnome.Mutter.ScreenCast", flags)?;

        Ok(conn)
    }
}

impl Session {
    pub fn new(
        id: CastSessionId,
        ipc_outputs: IpcOutputMap,
        to_srwm: smithay::reexports::calloop::channel::Sender<ScreenCastToSrwm>,
    ) -> Self {
        Self {
            id,
            ipc_outputs,
            streams: Arc::new(Mutex::new(vec![])),
            to_srwm,
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.to_srwm.send(ScreenCastToSrwm::StopCast {
            session_id: self.id,
        });
    }
}

impl Stream {
    fn new(
        id: CastStreamId,
        session_id: CastSessionId,
        target: StreamTarget,
        cursor_mode: CursorMode,
        to_srwm: smithay::reexports::calloop::channel::Sender<ScreenCastToSrwm>,
    ) -> Self {
        Self {
            id,
            session_id,
            target,
            cursor_mode,
            was_started: Arc::new(AtomicBool::new(false)),
            to_srwm,
        }
    }

    fn start(&self, ctxt: SignalEmitter<'static>) {
        if self.was_started.load(Ordering::SeqCst) {
            return;
        }

        let msg = ScreenCastToSrwm::StartCast {
            session_id: self.session_id,
            stream_id: self.id,
            target: self.target.make_id(),
            cursor_mode: self.cursor_mode,
            signal_ctx: ctxt,
        };

        if let Err(err) = self.to_srwm.send(msg) {
            tracing::warn!("error sending StartCast to srwm: {err:?}");
        }
    }
}

impl StreamTarget {
    fn make_id(&self) -> StreamTargetId {
        match self {
            StreamTarget::Output(output) => StreamTargetId::Output {
                name: output.name.clone(),
            },
            StreamTarget::Window { id } => StreamTargetId::Window { id: *id },
        }
    }
}
