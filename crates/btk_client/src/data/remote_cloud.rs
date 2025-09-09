use anyhow::Result;
use network_common::*;

use crate::network::NetworkConnection;

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
pub struct RemoteCloud {
    url: String,
    connection_maybe: Option<NetworkConnection>,
}

impl RemoteCloud {
    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &self.connection_maybe {
            connection.is_open().is_ok()
        } else {
            false
        }
    }

    fn send(&self, action: Action) -> Result<()> {
        if let Some(connection) = &self.connection_maybe {
            connection.write_connection(action);
            Ok(())
        } else {
            anyhow::bail!("NetworkManager: attempted to send without a connection");
        }
    }

    fn receive(&self) -> Result<Vec<Response>> {
        if let Some(connection) = &self.connection_maybe {
            Ok(connection.read_connection())
        } else {
            anyhow::bail!("NetworkManager: attempted to receive without a connection");
        }
    }
}
