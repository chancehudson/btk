mod cloud;

use std::path::PathBuf;

use anondb::Bytes;
use anondb::Journal;
use anyhow::Result;
use ml_dsa::KeyGen;
use ml_dsa::MlDsa87;
use serde::Deserialize;
use serde::Serialize;

/// We're going to need a few different databases.

/// Everything we need to interface with an encrypted cloud.
/// This includes local first mutations, handling differences with the remote cloud, persisting and
/// propagating.
pub struct LocalState {
    pub db: Journal,
    pub local_data_path: Option<PathBuf>,
}

const CLOUDS_TABLE: &str = "clouds_by_name";

/// Meta info about an encrypted cloud.
#[derive(Serialize, Deserialize)]
pub struct CloudMetadata {
    /// Public key that identifies the cloud.
    public_key: Vec<u8>,
    /// Private key that may mutate the cloud.
    private_key: [u8; 32],
    /// Latest known mutation index.
    latest_known_index: u64,
}

impl CloudMetadata {
    pub fn new() -> Self {
        let private_key: [u8; 32] = rand::random();
        let signer = MlDsa87::key_gen_internal(&private_key.into());
        Self {
            private_key,
            public_key: signer.verifying_key().encode().to_vec(),
            latest_known_index: 0,
        }
    }
}

impl LocalState {
    /// Retrieve all the encrypted clouds that are known to the client.
    pub fn list_clouds(&self) -> Result<Vec<CloudMetadata>> {
        let clouds = self
            .db
            .find_many::<Bytes, CloudMetadata, _>(CLOUDS_TABLE, |_, _| true)?
            .into_iter()
            .map(|(_k, v)| v)
            .collect::<Vec<_>>();
        Ok(clouds)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Self> {
        if let Some(project_dirs) = directories::ProjectDirs::from("org", "btkcloud", "btk_client")
        {
            let data_dir = project_dirs.data_local_dir();
            std::fs::create_dir_all(data_dir)?;
            Ok(Self {
                db: Journal::from(redb::Database::create(data_dir.join("local_data.redb"))?),
                local_data_path: Some(data_dir.into()),
            })
        } else {
            // unable to find a path for persistence, run in memory
            Ok(Self {
                db: Journal::in_memory(None)?,
                local_data_path: None,
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Result<Self> {
        Ok(Self {
            db: Journal::in_memory(None)?,
            local_data_path: None,
        })
    }
}
