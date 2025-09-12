use std::sync::Arc;

use anondb::Journal;
use anyhow::Result;
use network_common::*;

use crate::network::NetworkConnection;
use crate::tokio;

use super::Cloud;

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
#[derive(Clone)]
pub struct RemoteCloud {
    id: [u8; 32],
    http_url: String,
    ws_url: String,
    connection_maybe: Option<Arc<NetworkConnection>>,
    latest_synced_index: Option<u64>,
    cloud: Arc<Cloud>,
}

impl RemoteCloud {
    pub fn new(id: [u8; 32], ws_url: String, http_url: String, cloud: Arc<Cloud>) -> Self {
        let mut out = Self {
            connection_maybe: None,
            ws_url,
            http_url,
            id,
            latest_synced_index: None,
            cloud,
        };
        out.reconnect_if_needed();
        out
    }

    pub fn start_sync_loop(&self) {
        let cloud = self.cloud.clone();
        let http_url = self.http_url.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let base_url = reqwest::Url::parse(&http_url).unwrap();
            let mut latest_synced_index: Option<u64> = None;
            loop {
                if latest_synced_index.is_none() {
                    let mut url = base_url.join("/state").unwrap();
                    url.set_query(Some(&format!("cloud_id={}", cloud.id_hex())));
                    // let res = client.get(url).send().await?;
                    // if res.status().is_success() {
                    //     // let res_bytes = res.bytes()?;
                    // } else {
                    //     println!("failed to retrieve cloud state");
                    // }
                    tokio::spawn(async move {});
                }
            }
        });
    }

    pub fn tick(&self) -> Result<()> {
        if self.latest_synced_index.is_none() {
            // load the latest known index from the server
            return Ok(());
        }
        for response in self.receive()? {
            match response {
                Response::Pong => {
                    println!("pong");
                }
                Response::CloudMutated(index) => {}
                Response::Authenticated(cloud_id) => {}
            }
        }
        Ok(())
    }

    pub fn reconnect_if_needed(&mut self) {
        if !self.is_connected() {
            self.connection_maybe = Some(Arc::new(NetworkConnection::attempt_connection(
                self.ws_url.clone(),
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
