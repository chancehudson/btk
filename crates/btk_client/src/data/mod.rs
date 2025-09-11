mod cloud;
mod remote_cloud;

pub use cloud::Cloud;
pub use cloud::CloudMetadata;
pub use remote_cloud::RemoteCloud;

use std::collections::HashMap;
use std::path::PathBuf;

use anondb::Journal;
use anyhow::Result;

/// We're going to need a few different databases.

/// Misc data used by `LocalState` to maintain and operate `Cloud` instances.
/// Stored locally only.
const CLOUD_KEYS_TABLE: &str = "_______known_keys";

/// Key for the id of the last cloud that was active.
const ACTIVE_CLOUD_KEY: [u8; 32] = [0; 32];

/// Everything we need to interface with an encrypted cloud.
/// This includes local first mutations, handling differences with the remote cloud, persisting and
/// propagating.
pub struct LocalState {
    /// Application database, exists outside of all clouds.
    db: Journal,
    /// Path where persistent application data may be stored.
    pub local_data_dir: Option<PathBuf>,
    pub clouds: HashMap<[u8; 32], (Cloud, CloudMetadata)>,
    pub sorted_clouds: Vec<(Cloud, CloudMetadata)>,
    pub active_cloud_id: Option<[u8; 32]>,
}

impl LocalState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            db: if let Some(data_dir) = Self::local_data_dir()? {
                redb::Database::create(data_dir.join("local_data.redb"))?.into()
            } else {
                Journal::in_memory(None)?
            },
            local_data_dir: Self::local_data_dir()?,
            clouds: HashMap::default(),
            active_cloud_id: None,
            sorted_clouds: Vec::default(),
        })
    }

    /// Initialize `LocalState` using `self.db`.
    pub fn init(&mut self) -> Result<()> {
        self.load_clouds()?;

        self.active_cloud_id = self.db.get(CLOUD_KEYS_TABLE, &ACTIVE_CLOUD_KEY)?;

        Ok(())
    }

    pub fn load_clouds(&mut self) -> Result<()> {
        let data_dir_maybe = Self::local_data_dir()?;

        let mut next_clouds = HashMap::default();
        for key in self.cloud_keys()? {
            let cloud_id = Cloud::id_from_key(key.into());
            if let Some((cloud, _)) = self.clouds.get(&cloud_id) {
                next_clouds.insert(cloud_id, (cloud.clone(), cloud.load_metadata()?));
            } else {
                let cloud = Cloud::from_key(key.into(), data_dir_maybe.clone())?;
                next_clouds.insert(cloud_id, (cloud.clone(), cloud.load_metadata()?));
            }
        }
        self.clouds = next_clouds;
        self.sorted_clouds = self
            .clouds
            .values()
            .into_iter()
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
    pub fn create_cloud(&self) -> Result<Cloud> {
        let cloud = Cloud::new(Self::local_data_dir()?)?;
        cloud.set_metadata(CloudMetadata::create())?;
        self.db
            .insert(CLOUD_KEYS_TABLE, cloud.id(), cloud.private_key())?;
        Ok(cloud)
    }

    pub fn set_active_cloud(&mut self, id: [u8; 32]) -> Result<()> {
        self.db.insert(CLOUD_KEYS_TABLE, &ACTIVE_CLOUD_KEY, &id)?;
        self.active_cloud_id = Some(id);
        Ok(())
    }

    pub fn active_cloud(&self) -> Result<Option<&(Cloud, CloudMetadata)>> {
        if let Some(cloud_id) = self.active_cloud_id
            && let Some(cloud) = self.clouds.get(&cloud_id)
        {
            Ok(Some(cloud))
        } else {
            Ok(None)
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
