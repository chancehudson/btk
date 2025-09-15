use std::sync::Arc;
use std::sync::RwLock;

use anondb::Bytes;
use anyhow::Result;
use network_common::*;
use reqwest::StatusCode;

use crate::app::AppEvent;
use crate::network::NetworkConnection;

use super::Cloud;

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
#[derive(Clone)]
pub struct RemoteCloud {
    http_url: String,
    ws_url: String,
    connection_maybe: Option<Arc<NetworkConnection>>,
    pub(crate) cloud: Arc<Cloud>,
    latest_confirmed_index: Arc<RwLock<Option<u64>>>,
    initial_sync_complete: Arc<RwLock<bool>>,
}

impl RemoteCloud {
    pub fn new(_id: [u8; 32], ws_url: String, http_url: String, cloud: Arc<Cloud>) -> Self {
        let mut out = Self {
            connection_maybe: None,
            ws_url,
            http_url,
            cloud,
            latest_confirmed_index: Arc::new(RwLock::new(None)),
            initial_sync_complete: Arc::new(RwLock::new(false)),
        };
        out.reconnect_if_needed();
        out
    }

    /// A single synchronization tick. Should be a short lived task to advance the state of
    /// synchronization.
    pub async fn tick(
        &self,
        events_tx: flume::Sender<AppEvent>,
        sync_status_tx: flume::Sender<([u8; 32], String)>,
    ) -> Result<()> {
        let base_url = reqwest::Url::parse(&self.http_url)?;
        let journal_len = self.cloud.db.journal_tx_len()?;
        for i in 0..journal_len {
            if let Some(confirmed_index) = self.latest_confirmed_index.read().unwrap().clone() {
                if confirmed_index >= i {
                    continue;
                }
            }
            let tx = self.cloud.db.journal_tx_by_index(i)?;
            if tx.is_none() {
                anyhow::bail!("unable to find transaction in journal!");
            }
            let tx = tx.unwrap();
            // load the corresponding mutation from the server
            let mut url = base_url.join("/mutation")?;
            url.set_query(Some(&format!("cloud_id={}&index={i}", self.cloud.id_hex())));
            let res = reqwest::get(url).await?;

            if res.status().is_success() {
                let mutation = Bytes::from(res.bytes().await?.to_vec()).parse::<Mutation>()?;
                let (remote_tx, index) = self.cloud.decrypt_tx(mutation)?;
                assert_eq!(index, i, "index mismatch from remote");

                if remote_tx.hash()? == tx.hash()? {
                    // println!("hashes match!");
                    *self.latest_confirmed_index.write().unwrap() = Some(i);
                    sync_status_tx.send((
                        *self.cloud.id(),
                        format!("Confirmed {} of {}", i, journal_len),
                    ))?;
                    continue;
                }

                println!("cloud has diverged!");
                sync_status_tx.send((*self.cloud.id(), format!("Diverged at change {}", i)))?;
                // TODO: handle merge

                return Ok(());
            } else if res.status() == StatusCode::FAILED_DEPENDENCY {
                sync_status_tx.send((*self.cloud.id(), format!("Broadcasting change {}", i)))?;
                // send the mutation
                let mutation = self.cloud.encrypt_tx(tx.clone(), i)?;
                let mut url = base_url.join("/mutate")?;
                url.set_query(Some(&format!("cloud_id={}", self.cloud.id_hex(),)));
                let client = reqwest::Client::new();
                let res = client
                    .post(url)
                    .body(Bytes::encode(&mutation)?.to_vec())
                    .send()
                    .await?;
                if res.status().is_success() {
                    println!("successfully sent mutation {}", i);
                    *self.latest_confirmed_index.write().unwrap() = Some(i);
                    continue;
                } else {
                    println!("failed to send mutation: {:?}", res.status());
                    return Ok(());
                }
            } else {
                println!("unknown status from get request {:?}", res.status());
                break;
            }
        }

        if self.receive()?.len() == 0 && *self.initial_sync_complete.read().unwrap() {
            return Ok(());
        }

        let mut url = base_url.join("/state")?;
        url.set_query(Some(&format!("cloud_id={}", self.cloud.id_hex(),)));
        let res = reqwest::get(url).await?;
        let remote_index = if res.status().is_success() {
            Bytes::from(res.bytes().await?.to_vec()).parse::<u64>()?
        } else {
            println!("failed to get server state");
            return Ok(());
        };
        *self.initial_sync_complete.write().unwrap() = true;

        let mut current_index = journal_len;
        while remote_index > current_index {
            sync_status_tx.send((
                *self.cloud.id(),
                format!("Downloading change {}", current_index),
            ))?;
            // we're fully synced locally, now look for changes the server has but we don't
            let mut url = base_url.join("/mutation")?;
            url.set_query(Some(&format!(
                "cloud_id={}&index={}",
                self.cloud.id_hex(),
                current_index
            )));
            let res = reqwest::get(url).await?;
            if res.status().is_success() {
                // received a new change, apply it
                let mutation = Bytes::from(res.bytes().await?.to_vec()).parse::<Mutation>()?;
                let (remote_tx, _index) = self.cloud.decrypt_tx(mutation)?;
                self.cloud.db.append_tx(&remote_tx)?;
                *self.latest_confirmed_index.write().unwrap() = Some(current_index);
                events_tx.send(AppEvent::RemoteCloudUpdate(*self.cloud.id()))?;
            } else {
                sync_status_tx.send((
                    *self.cloud.id(),
                    format!("Error downloading change {}", current_index),
                ))?;
                break;
            }

            current_index += 1;
        }

        if current_index == remote_index {
            sync_status_tx.send((
                *self.cloud.id(),
                format!("Fully synchronized! ({}/{})", current_index, remote_index),
            ))?;
        }

        Ok(())
    }

    pub fn reconnect_if_needed(&mut self) {
        if !self.is_connected() {
            let mut full_url = reqwest::Url::parse(&self.ws_url).expect("failed to parse ws url");
            full_url.set_query(Some(&format!("cloud_id={}", self.cloud.id_hex(),)));
            self.connection_maybe = Some(Arc::new(NetworkConnection::attempt_connection(
                full_url.to_string(),
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
