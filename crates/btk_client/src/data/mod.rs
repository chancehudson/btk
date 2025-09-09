mod cloud;
mod remote_cloud;

use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;

use anondb::Journal;
use anyhow::Result;
use cloud::Cloud;

/// We're going to need a few different databases.

const CLOUD_KEYS_TABLE: &str = "_______known_keys";

/// Everything we need to interface with an encrypted cloud.
/// This includes local first mutations, handling differences with the remote cloud, persisting and
/// propagating.
pub struct LocalState {
    /// Application database, exists outside of all clouds.
    db: Journal,
    /// Path where persistent application data may be stored.
    pub local_data_dir: Option<PathBuf>,
    pub clouds: HashMap<[u8; 32], Cloud>,
    pub active_cloud_id: Option<[u8; 32]>,
}

impl LocalState {
    /// Retrieve all the encrypted clouds that we know how to decrypt.
    fn cloud_keys(&self) -> Result<Vec<[u8; 32]>> {
        Ok(self
            .db
            .find_many::<[u8; 32], [u8; 32], _>(CLOUD_KEYS_TABLE, |_, _| true)?
            .into_iter()
            .map(|(_k, v)| v)
            .collect())
    }

    pub fn load_clouds(&mut self) -> Result<Vec<&Cloud>> {
        mem::take(&mut self.clouds);
        let data_dir_maybe = Self::local_data_dir()?;
        for cloud in self
            .cloud_keys()?
            .into_iter()
            .map(|key| Cloud::from_key(key.into(), data_dir_maybe.clone()))
            .collect::<Result<Vec<Cloud>>>()?
        {
            self.clouds.insert(*cloud.id(), cloud);
        }
        Ok(self.clouds.values().into_iter().collect())
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
    /// Formally a cloud is a `Vec<network_common::Mutation>`.
    ///
    pub fn create_cloud(&self) -> Result<Cloud> {
        let cloud = Cloud::new(Self::local_data_dir()?)?;
        self.db
            .insert(CLOUD_KEYS_TABLE, cloud.id(), cloud.private_key())?;
        Ok(cloud)
    }

    pub fn set_active_cloud(&mut self, id: [u8; 32]) {
        self.active_cloud_id = Some(id);
    }

    pub fn active_cloud(&self) -> Result<&Cloud> {
        if let Some(cloud_id) = self.active_cloud_id
            && let Some(cloud) = self.clouds.get(&cloud_id)
        {
            Ok(cloud)
        } else {
            anyhow::bail!("active cloud not found");
        }
    }

    pub fn new() -> Result<Self> {
        let mut out = if let Some(data_dir) = Self::local_data_dir()? {
            Self {
                db: redb::Database::create(data_dir.join("local_data.redb"))?.into(),
                local_data_dir: Some(data_dir.into()),
                clouds: HashMap::default(),
                active_cloud_id: None,
            }
        } else {
            // unable to find a path for persistence, run in memory
            Self {
                db: Journal::in_memory(None)?,
                local_data_dir: None,
                clouds: HashMap::default(),
                active_cloud_id: None,
            }
        };
        out.load_clouds()?;
        Ok(out)
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
