pub mod mutter_screen_cast;
pub mod mutter_service_channel;
use zbus::blocking::Connection;
use zbus::object_server::Interface;
pub mod gnome_shell_introspect;
pub mod mutter_display_config;
pub(crate) mod screencasting;

pub use mutter_screen_cast::ScreenCast;

pub trait Start: Interface {
    fn start(self) -> anyhow::Result<Connection>;
}

pub fn try_start<I: Start>(iface: I) -> Option<Connection> {
    match iface.start() {
        Ok(conn) => Some(conn),
        Err(err) => {
            tracing::warn!("error starting {}: {err:?}", I::name());
            None
        }
    }
}

/// Start all D-Bus servers for the compositor.
/// Returns the ScreenCast connection handle (if any).
pub fn start_screen_cast(screen_cast: ScreenCast) -> Option<Connection> {
    try_start(screen_cast)
}
