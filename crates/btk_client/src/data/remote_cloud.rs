use std::sync::Arc;

use anyhow::Result;
use network_common::*;

use crate::network::NetworkConnection;

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
#[derive(Clone)]
pub struct RemoteCloud {
    url: String,
    connection_maybe: Option<Arc<NetworkConnection>>,
}

impl RemoteCloud {
    pub fn new(url: String) -> Self {
        Self {
            connection_maybe: Some(Arc::new(NetworkConnection::attempt_connection(url.clone()))),
            url,
        }
    }

    pub fn reconnect_if_needed(&mut self) {
        if !self.is_connected() {
            self.connection_maybe = Some(Arc::new(NetworkConnection::attempt_connection(
                self.url.clone(),
            )));
        }
    }

    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &self.connection_maybe {
            connection.is_open().is_ok()
        } else {
            false
        }
    }

    pub fn send(&self, action: Action) -> Result<()> {
        if let Some(connection) = &self.connection_maybe {
            connection.write_connection(action);
            Ok(())
        } else {
            anyhow::bail!("NetworkManager: attempted to send without a connection");
        }
    }

    pub fn receive(&self) -> Result<Vec<Response>> {
        if let Some(connection) = &self.connection_maybe {
            Ok(connection.read_connection())
        } else {
            anyhow::bail!("NetworkManager: attempted to receive without a connection");
        }
    }
}
