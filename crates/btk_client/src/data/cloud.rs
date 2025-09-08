use std::path::PathBuf;

use anondb::Bytes;
use anondb::Journal;
use anondb::TransactionOperation;
use anyhow::Result;
use chacha20::ChaCha20;
use chacha20::cipher::KeyIvInit;
use chacha20::cipher::StreamCipher;
use ml_dsa::KeyGen;
use network_common::*;

use crate::network::NetworkConnection;

use ml_dsa::MlDsa87;
use ml_dsa::signature::Signer;

/// A trustlessly replicated Anondb instance with simple conflict resolution for realtime
/// collaboration among keyholders.
pub struct Cloud {
    url: String,
    connection_maybe: Option<NetworkConnection>,
    local_db: Journal,
    local_data_path: Option<PathBuf>,
    prvkey: [u8; 32],
    prvkey_hash: [u8; 32],
    pubkey_hash: [u8; 32],
}

impl Cloud {
    /// Accept an anondb transaction and create a trustless representation.
    fn encrypt_tx(&self, index: u64, transaction: Vec<TransactionOperation>) -> Result<Mutation> {
        let signer = MlDsa87::key_gen_internal(&self.prvkey.into());

        let salt: [u8; 32] = rand::random();

        let mutation_key_preimage = Bytes::encode(&(&self.prvkey_hash, index, &salt))?;
        let mutation_key = blake3::hash(&mutation_key_preimage.as_slice())
            .as_bytes()
            .to_vec();

        // now we can encrypt the transaction data

        let mut tx_bytes: Vec<u8> = Bytes::encode(&transaction)?.into();
        let mut chacha = ChaCha20::new(
            mutation_key.as_slice().into(),
            // we can safely choose 0 as the nonce because the encryption key is salted with a
            // strong random value preventing any encryption key from being used twice.
            vec![0_u8; 32].as_slice().into(),
        );
        chacha.apply_keystream(&mut tx_bytes);
        drop(chacha);

        // tx_bytes are now encrypted

        let signature = signer.sign(&tx_bytes).encode().to_vec();

        Ok(Mutation {
            index,
            data: tx_bytes,
            signature,
            public_key_hash: self.pubkey_hash,
            public_key: if index == 0 {
                Some(signer.verifying_key().encode().to_vec())
            } else {
                None
            },
            salt,
            mutation_key: None,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(url: String, prvkey: [u8; 32]) -> Result<Self> {
        let signer = MlDsa87::key_gen_internal(&prvkey.into());
        let prvkey_hash = blake3::hash(&prvkey).as_bytes().clone();
        let pubkey_hash = blake3::hash(&signer.verifying_key().encode().to_vec())
            .as_bytes()
            .clone();
        if let Some(project_dirs) = directories::ProjectDirs::from("org", "btkcloud", "btk_client")
        {
            let data_dir = project_dirs.data_local_dir();
            std::fs::create_dir_all(data_dir)?;
            Ok(Self {
                prvkey,
                prvkey_hash,
                pubkey_hash,
                url,
                connection_maybe: None,
                local_db: Journal::from(redb::Database::create(data_dir.join("local_data.redb"))?),
                local_data_path: Some(data_dir.into()),
            })
        } else {
            // unable to find a path for persistence, run in memory
            Ok(Self {
                prvkey,
                prvkey_hash,
                pubkey_hash,
                url,
                connection_maybe: None,
                local_db: Journal::in_memory(None)?,
                local_data_path: None,
            })
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(url: String, prvkey: [u8; 32]) -> Result<Self> {
        let signer = MlDsa87::key_gen_internal(&prvkey.into());
        let prvkey_hash = blake3::hash(&prvkey).as_bytes().clone();
        let pubkey_hash = blake3::hash(&signer.verifying_key().encode().to_vec())
            .as_bytes()
            .clone();
        Ok(Self {
            prvkey,
            prvkey_hash,
            pubkey_hash,
            url,
            connection_maybe: None,
            local_db: Journal::in_memory(None)?,
            local_data_path: None,
        })
    }

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
