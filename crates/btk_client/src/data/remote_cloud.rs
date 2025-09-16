use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use anondb::Bytes;
use anondb::Journal;
use anyhow::Result;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use web_time::Instant;

use crate::app::AppEvent;
use crate::network::NetworkConnection;
use network_common::*;

use super::Cloud;

const DEFAULT_SYNC_HTTP_URL: &str = "https://btk_worker.jchancehud.workers.dev";
const DEFAULT_SYNC_WS_URL: &str = "wss://btk_worker.jchancehud.workers.dev";

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudSyncState {
    pub http_url: String,
    pub ws_url: String,
    pub latest_confirmed_index: Option<u64>,
    pub synchronization_enabled: bool,
}

impl Default for CloudSyncState {
    fn default() -> Self {
        Self {
            http_url: DEFAULT_SYNC_HTTP_URL.to_string(),
            ws_url: DEFAULT_SYNC_WS_URL.to_string(),
            latest_confirmed_index: None,
            synchronization_enabled: true,
        }
    }
}

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
#[derive(Clone)]
pub struct RemoteCloud {
    ctx: egui::Context,
    db: Journal,
    sync_state: Arc<RwLock<CloudSyncState>>,
    connection_maybe: Arc<RwLock<Option<NetworkConnection>>>,
    pub(crate) cloud: Arc<Cloud>,
    initial_sync_complete: Arc<RwLock<bool>>,
    last_keepalive: Arc<RwLock<Instant>>,
}

impl RemoteCloud {
    pub fn new(
        data_dir_maybe: Option<PathBuf>,
        cloud: Arc<Cloud>,
        ctx: egui::Context,
    ) -> Result<Self> {
        let db = if let Some(data_dir) = data_dir_maybe {
            let filepath = data_dir.join(format!("sync-{}.redb", cloud.id_hex()));
            Journal::at_path(&filepath)?
        } else {
            Journal::in_memory(None)?
        };
        let sync_state = Arc::new(RwLock::new(
            db.get::<(), CloudSyncState>("sync_state", &())?
                .unwrap_or_default(),
        ));
        Ok(Self {
            sync_state,
            ctx,
            connection_maybe: Arc::new(RwLock::new(None)),
            db,
            cloud,
            initial_sync_complete: Arc::new(RwLock::new(false)),
            last_keepalive: Arc::new(RwLock::new(Instant::now())),
        })
    }

    pub fn set_synchronization_enabled(&self, enabled: bool) -> Result<()> {
        self.sync_state.write().unwrap().synchronization_enabled = enabled;
        if !enabled {
            *self.initial_sync_complete.write().unwrap() = false;
        }
        self.write_sync_state()
    }

    pub fn synchronization_enabled(&self) -> bool {
        self.sync_state.read().unwrap().synchronization_enabled
    }

    fn set_latest_confirmed_index(&self, new_index: u64) -> Result<()> {
        self.sync_state.write().unwrap().latest_confirmed_index = Some(new_index);
        self.write_sync_state()
    }

    pub fn latest_confirmed_index(&self) -> Option<u64> {
        self.sync_state.read().unwrap().latest_confirmed_index
    }

    pub fn http_url(&self) -> String {
        self.sync_state.read().unwrap().http_url.clone()
    }

    pub fn ws_url(&self) -> String {
        self.sync_state.read().unwrap().ws_url.clone()
    }

    pub fn write_sync_state(&self) -> Result<()> {
        self.db
            .insert("sync_state", &(), &*self.sync_state.read().unwrap())?;
        Ok(())
    }

    /// A single synchronization tick. Should be a short lived task to advance the state of
    /// synchronization.
    pub async fn tick(
        &self,
        events_tx: flume::Sender<AppEvent>,
        sync_status_tx: flume::Sender<([u8; 32], String)>,
    ) -> Result<()> {
        if !self.synchronization_enabled() {
            self.ctx.request_repaint();
            sync_status_tx.send((*self.cloud.id(), format!("Synchronization disabled")))?;
            *self.connection_maybe.write().unwrap() = None;
            return Ok(());
        }
        self.reconnect_if_needed();
        if Instant::now()
            .duration_since(*self.last_keepalive.read().unwrap())
            .as_secs()
            > 20
        {
            *self.last_keepalive.write().unwrap() = Instant::now();
            self.send(Action::Ping)?;
        }

        let base_url = reqwest::Url::parse(&self.http_url())?;
        let journal_len = self.cloud.db.journal_tx_len()?;
        for i in 0..journal_len {
            if let Some(confirmed_index) = self.latest_confirmed_index() {
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
                    self.set_latest_confirmed_index(i)?;
                    self.ctx.request_repaint();
                    sync_status_tx.send((
                        *self.cloud.id(),
                        format!("Confirmed {} of {}", i, journal_len),
                    ))?;
                    continue;
                }

                println!("cloud has diverged!");
                self.ctx.request_repaint();
                sync_status_tx.send((
                    *self.cloud.id(),
                    format!("Diverged at mutation index #{}", i),
                ))?;
                // TODO: handle merge

                return Ok(());
            } else if res.status() == StatusCode::FAILED_DEPENDENCY {
                self.ctx.request_repaint();
                sync_status_tx.send((
                    *self.cloud.id(),
                    format!("Broadcasting mutation #{}", i + 1),
                ))?;
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
                    self.set_latest_confirmed_index(i)?;
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

        if self
            .receive()?
            .iter()
            .filter(|v| !matches!(v, Response::Pong))
            .collect::<Vec<_>>()
            .len()
            == 0
            && *self.initial_sync_complete.read().unwrap()
        {
            self.ctx.request_repaint();
            sync_status_tx.send((
                *self.cloud.id(),
                format!("Fully synchronized! ({}/{})", journal_len, journal_len),
            ))?;
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
            self.ctx.request_repaint();
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
                self.set_latest_confirmed_index(current_index)?;
                events_tx.send(AppEvent::RemoteCloudUpdate(*self.cloud.id()))?;
            } else {
                self.ctx.request_repaint();
                sync_status_tx.send((
                    *self.cloud.id(),
                    format!("Error downloading change {}", current_index),
                ))?;
                break;
            }

            current_index += 1;
        }

        if current_index == remote_index {
            self.ctx.request_repaint();
            sync_status_tx.send((
                *self.cloud.id(),
                format!("Fully synchronized! ({}/{})", current_index, remote_index),
            ))?;
        }

        Ok(())
    }

    pub fn reconnect_if_needed(&self) {
        if !self.is_connected() {
            let mut full_url = reqwest::Url::parse(&self.ws_url()).expect("failed to parse ws url");
            full_url.set_query(Some(&format!("cloud_id={}", self.cloud.id_hex(),)));
            *self.connection_maybe.write().unwrap() =
                Some(NetworkConnection::attempt_connection(full_url.to_string()));
            *self.last_keepalive.write().unwrap() = Instant::now();
        }
    }

    pub fn is_connected(&self) -> bool {
        if let Some(connection) = &*self.connection_maybe.read().unwrap() {
            connection.is_open().is_ok()
        } else {
            false
        }
    }

    pub fn send(&self, action: Action) -> Result<()> {
        if let Some(connection) = &*self.connection_maybe.read().unwrap() {
            connection.write_connection(action);
            Ok(())
        } else {
            anyhow::bail!("NetworkManager: attempted to send without a connection");
        }
    }

    pub fn receive(&self) -> Result<Vec<Response>> {
        if let Some(connection) = &*self.connection_maybe.read().unwrap() {
            Ok(connection.read_connection())
        } else {
            anyhow::bail!("NetworkManager: attempted to receive without a connection");
        }
    }
}
