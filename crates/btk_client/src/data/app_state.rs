use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use anondb::Journal;
use anyhow::Result;
use web_time::Duration;

use crate::app::ActionRequest;
use crate::app::AppEvent;
use crate::data::Cloud;
use crate::data::CloudMetadata;
use crate::data::RemoteCloud;
use crate::tokio;

/// We're going to need a few different databases.

/// Misc data used by `LocalState` to maintain and operate `Cloud` instances.
/// Stored locally only.
const CLOUD_KEYS_TABLE: &str = "_______known_keys";

/// Key for the id of the last cloud that was active.
const ACTIVE_CLOUD_KEY: [u8; 32] = [0; 32];

/// Everything we need to interface with an encrypted cloud.
/// This includes local first mutations, handling differences with the remote cloud, persisting and
/// propagating.
///
/// A state object that is accessible in all applets.
pub struct AppState {
    pub ctx: egui::Context,
    pub pending_events: (flume::Sender<AppEvent>, flume::Receiver<AppEvent>),
    pub pending_requests: (flume::Sender<ActionRequest>, flume::Receiver<ActionRequest>),
    pub sync_status: (
        flume::Sender<([u8; 32], String)>,
        flume::Receiver<([u8; 32], String)>,
    ),
    /// Application database, exists outside of all clouds.
    db: Journal,
    /// Path where persistent application data may be stored.
    pub clouds: RwLock<HashMap<[u8; 32], (Arc<Cloud>, CloudMetadata)>>,
    pub remote_clouds: Arc<RwLock<HashMap<[u8; 32], RemoteCloud>>>,
    pub sorted_clouds: Vec<(Arc<Cloud>, CloudMetadata)>,
    pub active_cloud_id: Option<[u8; 32]>,
}

impl AppState {
    pub fn drain_pending_app_events(&self) -> Vec<AppEvent> {
        self.pending_events.1.drain().collect()
    }

    pub fn switch_cloud(&self, id: [u8; 32]) {
        self.pending_requests
            .0
            .send(ActionRequest::SwitchCloud(id))
            .expect("failed to send app request");
    }

    pub fn reload_clouds(&self) {
        self.pending_requests
            .0
            .send(ActionRequest::LoadClouds)
            .expect("failed to send app request");
    }

    pub fn drain_pending_app_requests(&self) -> Vec<ActionRequest> {
        self.pending_requests.1.drain().collect()
    }

    pub fn new(ctx: egui::Context) -> Result<Self> {
        Ok(Self {
            ctx,
            pending_events: flume::unbounded(),
            pending_requests: flume::unbounded(),
            sync_status: flume::unbounded(),
            db: if let Some(data_dir) = Self::local_data_dir()? {
                redb::Database::create(data_dir.join("local_data.redb"))?.into()
            } else {
                Journal::in_memory(None)?
            },
            clouds: RwLock::new(HashMap::default()),
            active_cloud_id: None,
            sorted_clouds: Vec::default(),
            remote_clouds: Arc::new(RwLock::new(HashMap::default())),
        })
    }

    /// Initialize `LocalState` using `self.db`.
    pub fn init(&mut self) -> Result<()> {
        self.load_clouds()?;

        self.active_cloud_id = self.db.get(CLOUD_KEYS_TABLE, &ACTIVE_CLOUD_KEY)?;

        let remote_clouds = self.remote_clouds.clone();
        let events_tx = self.pending_events.0.clone();
        let sync_status_tx = self.sync_status.0.clone();
        tokio::spawn(async move {
            loop {
                let remotes = remote_clouds
                    .read()
                    .unwrap()
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                for remote in remotes {
                    if let Err(e) = remote.tick(events_tx.clone(), sync_status_tx.clone()).await {
                        println!("Error ticking remote! {:?}", e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    pub fn load_clouds(&mut self) -> Result<()> {
        let data_dir_maybe = Self::local_data_dir()?;

        let mut next_cloud_ids = HashSet::<[u8; 32]>::default();
        for key in self.cloud_keys()? {
            let cloud_id = Cloud::id_from_key(key.into());
            next_cloud_ids.insert(cloud_id);
            let mut clouds = self.clouds.write().unwrap();
            if let Some((cloud, _)) = clouds.get(&cloud_id).cloned() {
                clouds.insert(cloud_id, (cloud.clone(), cloud.load_metadata()?));
            } else {
                let cloud = Arc::new(Cloud::from_key(key.into(), data_dir_maybe.clone())?);
                clouds.insert(cloud_id, (cloud.clone(), cloud.load_metadata()?));
            }
        }
        self.clouds
            .write()
            .unwrap()
            .retain(|cloud_id, _| next_cloud_ids.contains(cloud_id));

        for (cloud, metadata) in self.clouds.read().unwrap().values() {
            if self.remote_clouds.read().unwrap().get(cloud.id()).is_none() {
                println!("opening connection for cloud {}", metadata.name);
                self.remote_clouds.write().unwrap().insert(
                    *cloud.id(),
                    RemoteCloud::new(Self::local_data_dir()?, cloud.clone(), self.ctx.clone())?,
                );
            }
        }
        self.remote_clouds
            .write()
            .unwrap()
            .retain(|k, _| self.clouds.read().unwrap().contains_key(k));
        self.sorted_clouds = self
            .clouds
            .write()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        self.sorted_clouds.sort_by(|(_, first), (_, second)| {
            if first.created_at == second.created_at {
                first.name.cmp(&second.name)
            } else {
                first.created_at.cmp(&second.created_at)
            }
        });
        Ok(())
    }

    /// Create a new encrypted cloud. This is a local keypair keyed
    /// to an entry in the database.
    ///
    /// The database is interpreted/operated by the application
    /// arbitrarily.
    ///
    /// The database is journaled by tracking redb k/v operations
    /// and replicating them on remote instances, behind chacha20
    /// encryption. A syntax for expressing queries
    /// (like MongoDB/SQL) can trivially be built around this.
    ///
    pub fn create_cloud(&self) -> Result<()> {
        let cloud = Cloud::new(Self::local_data_dir()?)?;
        let metadata = CloudMetadata::create();
        cloud.set_metadata(metadata.clone())?;
        self.db
            .insert(CLOUD_KEYS_TABLE, cloud.id(), cloud.private_key())?;
        self.clouds
            .write()
            .unwrap()
            .insert(*cloud.id(), (Arc::new(cloud), metadata));
        Ok(())
    }

    /// Returns the new cloud id
    pub fn import_cloud(&self, key_str: &str) -> Result<[u8; 32]> {
        let key_vec = hex::decode(key_str.trim())?;
        if key_vec.len() != 32 {
            anyhow::bail!("Key is not correct length");
        }
        let mut private_key = <[u8; 32]>::default();
        private_key.copy_from_slice(&key_vec);
        let cloud = Cloud::from_key(private_key, Self::local_data_dir()?)?;
        self.db
            .insert(CLOUD_KEYS_TABLE, cloud.id(), cloud.private_key())?;
        Ok(*cloud.id())
    }

    pub fn set_active_cloud(&mut self, id: [u8; 32]) -> Result<()> {
        self.db.insert(CLOUD_KEYS_TABLE, &ACTIVE_CLOUD_KEY, &id)?;
        self.active_cloud_id = Some(id);
        Ok(())
    }

    pub fn cloud_by_id(&self, cloud_id: &[u8; 32]) -> Option<(Arc<Cloud>, CloudMetadata)> {
        self.clouds.read().unwrap().get(cloud_id).cloned()
    }

    pub fn active_cloud(&self) -> Option<(Arc<Cloud>, CloudMetadata)> {
        if let Some(cloud_id) = self.active_cloud_id {
            self.cloud_by_id(&cloud_id)
        } else {
            None
        }
    }

    /// Retrieve all the encrypted clouds that we know how to decrypt.
    fn cloud_keys(&self) -> Result<Vec<[u8; 32]>> {
        Ok(self
            .db
            .find_many::<[u8; 32], [u8; 32], _>(CLOUD_KEYS_TABLE, |_, _| true)?
            .into_iter()
            .filter(|(k, _v)| k != &ACTIVE_CLOUD_KEY)
            .map(|(_k, v)| v)
            .collect())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn local_data_dir() -> Result<Option<PathBuf>> {
        if let Some(project_dirs) = directories::ProjectDirs::from("org", "btkcloud", "btk_client")
        {
            let data_dir = project_dirs.data_local_dir();
            std::fs::create_dir_all(data_dir)?;
            Ok(Some(data_dir.into()))
        } else {
            Ok(None)
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn local_data_dir() -> Result<Option<PathBuf>> {
        Ok(None)
    }
}
