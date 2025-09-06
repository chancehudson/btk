use std::path::PathBuf;

use anyhow::Result;
use redb::Database;

/// We're going to need a few different databases.
///
/// First we have the local_state db. This tracks what clouds we know about and have a local
/// representation of. The application loads and opens the local_state db (maybe password
/// encrypted). Users should be able to view, create, and switch clouds.
///
/// Next we have cloud_state databases. Each of these represents a holistic view of an encrypted
/// cloud. We need to persist the journal somehow, probably outside of redb. These may simply be a
/// series of files?
///
///

/// Everything we need to interface with an encrypted cloud.
/// This includes local first mutations, handling differences with the remote cloud, persisting and
/// propagating.
pub struct LocalState {
    pub db: Database,
    pub local_data_path: Option<PathBuf>,
}

/// Meta info about the cloud.
pub struct CloudMetadata {
    public_key: Vec<u8>,
}

impl LocalState {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Self> {
        if let Some(project_dirs) = directories::ProjectDirs::from("org", "btkcloud", "btk_client")
        {
            let data_dir = project_dirs.data_local_dir();
            std::fs::create_dir_all(data_dir)?;
            Ok(Self {
                db: Database::create(data_dir.join("local_data.redb"))?,
                local_data_path: Some(data_dir.into()),
            })
        } else {
            // unable to find a path for persistence, run in memory
            let db =
                redb::Builder::new().create_with_backend(redb::backends::InMemoryBackend::new())?;
            Ok(Self {
                db,
                local_data_path: None,
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Result<Self> {
        let db = redb::Builder::new().create_with_backend(redb::backends::InMemoryBackend::new())?;
        Ok(Self {
            db,
            local_data_path: None,
        })
    }
}
