use anyhow::Result;

use network_common::*;

#[cfg(not(target_arch = "wasm32"))]
mod network_native;
#[cfg(target_arch = "wasm32")]
mod network_wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use network_native::NetworkConnection;
#[cfg(target_arch = "wasm32")]
pub use network_wasm::NetworkConnection;

/// Abstraction around a concrete network connection. Handles reconnect logic, send/receive,
/// switching servers.
#[derive(Default)]
pub struct NetworkManager {
    active_url: String,
    connection_maybe: Option<NetworkConnection>,
}

impl NetworkManager {
    /// Create a new instance of a network manager and attempt to connect.
    pub fn new(url: &str) -> Self {
        Self {
            active_url: url.into(),
            connection_maybe: Some(NetworkConnection::attempt_connection(url.into())),
        }
    }

    pub fn url(&self) -> &str {
        &self.active_url
    }

    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &self.connection_maybe {
            connection.is_open().is_ok()
        } else {
            false
        }
    }

    pub fn send(&self) -> Result<()> {
        if let Some(connection) = &self.connection_maybe {
            // connection.write_connection(action);
            unimplemented!();
        } else {
            anyhow::bail!("NetworkManager: attempted to send without a connection");
        }
    }

    pub fn receive(&self) -> Result<Vec<Response>> {
        if let Some(connection) = &self.connection_maybe {
            // connection.write_connection(action);
            Ok(connection.read_connection())
        } else {
            anyhow::bail!("NetworkManager: attempted to receive without a connection");
        }
    }
}
