use std::sync::Arc;

use anondb::Journal;
use anyhow::Result;
use network_common::*;

use crate::network::NetworkConnection;

#[derive(Default, Clone)]
pub struct CloudSyncState {
    /// The latest index we've confirmed matches with the remote cloud. We trust the remote cloud
    /// not to alter history.
    pub latest_synced_index: u64,
}

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
#[derive(Clone)]
pub struct RemoteCloud {
    /// Local sync state. Tracks latest mutation known to the server.
    local_db: Journal,
    id: [u8; 32],
    url: String,
    connection_maybe: Option<Arc<NetworkConnection>>,
}

impl RemoteCloud {
    pub fn new(id: [u8; 32], url: String) -> Result<Self> {
        Ok(Self {
            connection_maybe: Some(Arc::new(NetworkConnection::attempt_connection(url.clone()))),
            url,
            id,
            local_db: if let Some(data_dir) = crate::data::LocalState::local_data_dir()? {
                let filepath = data_dir.join(hex::encode(id) + "_sync.redb");
                Journal::at_path(&filepath)?
            } else {
                Journal::in_memory(None)?
            },
        })
    }

    pub fn tick(&self) -> Result<()> {
        for response in self.receive()? {
            match response {
                Response::Pong => {
                    println!("pong");
                }
                Response::CloudMutated(index, mutation_hash) => {}
                Response::Authenticated(cloud_id) => {}
                Response::CloudMutation(mutation) => {
                    // ingest the mutation
                }
            }
        }
        Ok(())
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
