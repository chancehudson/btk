use std::path::PathBuf;

use anondb::Bytes;
use anondb::Journal;
use anondb::JournalTransaction;
use anyhow::Result;
use chacha20::ChaCha20;
use chacha20::cipher::KeyIvInit;
use chacha20::cipher::StreamCipher;
use ml_dsa::KeyGen;
use ml_dsa::MlDsa87;
use ml_dsa::signature::SignerMut;
use network_common::Action;
use network_common::Response;
use serde::Deserialize;
use serde::Serialize;
use web_time::SystemTime;

use network_common::Mutation;

use super::remote_cloud::RemoteCloud;

const CLOUD_TABLE: &str = "_______cloud_data";
const METADATA_KEY: &str = "metadata";

#[derive(Serialize, Deserialize, Clone)]
pub struct CloudMetadata {
    pub created_at: u64,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub remote_url: Option<String>,
}

impl Default for CloudMetadata {
    fn default() -> Self {
        let mut generator = names::Generator::default();
        Self {
            created_at: SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
            name: generator.next().unwrap_or_default(),
            description: String::default(),
            remote_url: None,
        }
    }
}

/// Meta info about an encrypted cloud.
#[derive(Clone)]
pub struct Cloud {
    pub metadata: CloudMetadata,
    /// Public key that identifies the cloud.
    public_key: Vec<u8>,
    /// Private key that may mutate the cloud.
    private_key: [u8; 32],
    /// Latest known mutation index.
    latest_known_index: u64,
    pub db: Journal,
    id: [u8; 32],
    remote: Option<RemoteCloud>,
    filepath: Option<PathBuf>,
}

impl Cloud {
    pub(crate) fn private_key(&self) -> &[u8; 32] {
        &self.private_key
    }

    pub fn id(&self) -> &[u8; 32] {
        &self.id
    }

    pub fn id_hex(&self) -> String {
        self.id
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    pub fn filepath(&self) -> Option<&PathBuf> {
        self.filepath.as_ref()
    }

    pub fn set_metadata(&mut self, metadata: CloudMetadata) -> Result<()> {
        self.db
            .insert(CLOUD_TABLE, &METADATA_KEY.to_string(), &metadata)?;
        self.metadata = metadata;
        if let Some(remote_url) = &self.metadata.remote_url {
            self.remote = Some(RemoteCloud::new(remote_url.clone()));
        }
        Ok(())
    }

    /// Called each frame
    pub fn update(&mut self) -> Result<()> {
        if let Some(remote) = self.remote.as_ref() {
            for response in remote.receive()? {
                match response {
                    Response::Pong => {}
                    Response::CloudMutated(index, cloud_id) => {
                        // if we already have this mutation ignore it
                        let db_state = self.db.get_state()?;
                        if index >= db_state.next_tx_index {
                            // we want to download this change
                            remote.send(Action::GetMutation(cloud_id, index))?;
                        }
                    }
                    Response::Authenticated(_) => {}
                    Response::CloudMutation(mutation) => {
                        let db_state = self.db.get_state()?;
                        if mutation.index == db_state.next_tx_index {
                            let tx = self.decrypt_tx(mutation)?;
                            self.db.append_tx(&tx.operations)?;
                        } else {
                            println!("received out of order mutation");
                        }
                    }
                }
            }
            for tx in self.db.drain_transactions()? {
                let mutation = self.encrypt_tx(tx)?;
                #[cfg(debug_assertions)]
                {
                    if let Err(e) = mutation.verify(self.public_key.clone()) {
                        println!("Failed to verify mutation: {:?}", e);
                    }
                }
                remote.send(Action::MutateCloud(mutation))?;
            }
        }
        Ok(())
    }

    pub fn init(&mut self) -> Result<()> {
        let metadata =
            if let Some(metadata) = self.db.get(CLOUD_TABLE, &METADATA_KEY.to_string())? {
                metadata
            } else {
                let metadata = CloudMetadata::default();
                self.db
                    .insert(CLOUD_TABLE, &METADATA_KEY.to_string(), &metadata)?;
                metadata
            };
        self.metadata = metadata;
        Ok(())
    }

    pub fn new(data_dir_maybe: Option<PathBuf>) -> Result<Self> {
        Self::from_key(rand::random(), data_dir_maybe)
    }

    pub fn id_from_key(private_key: [u8; 32]) -> [u8; 32] {
        let signer = MlDsa87::key_gen_internal(&private_key.into());
        let public_key = signer.verifying_key().encode().to_vec();
        let id: [u8; 32] = blake3::hash(&public_key).into();
        id
    }

    pub fn from_key(private_key: [u8; 32], data_dir_maybe: Option<PathBuf>) -> Result<Self> {
        let signer = MlDsa87::key_gen_internal(&private_key.into());
        let public_key = signer.verifying_key().encode().to_vec();
        let id: [u8; 32] = blake3::hash(&public_key).into();
        let hex_string = hex::encode(&id) + ".redb";
        let (db, filepath_maybe) = if let Some(data_dir) = data_dir_maybe {
            let filepath = data_dir.join(hex_string);
            (Journal::at_path(&filepath)?, Some(filepath))
        } else {
            (Journal::in_memory(None)?, None)
        };
        let metadata = if let Some(metadata) = db.get(CLOUD_TABLE, &METADATA_KEY.to_string())? {
            metadata
        } else {
            let metadata = CloudMetadata::default();
            db.insert(CLOUD_TABLE, &METADATA_KEY.to_string(), &metadata)?;
            metadata
        };
        let remote = if let Some(remote_url) = &metadata.remote_url {
            Some(RemoteCloud::new(remote_url.clone()))
        } else {
            None
        };

        Ok(Self {
            id,
            db,
            filepath: filepath_maybe,
            private_key,
            public_key,
            latest_known_index: 0,
            remote,
            metadata,
        })
    }

    fn decrypt_tx(&self, mutation: Mutation) -> Result<JournalTransaction> {
        let signer = MlDsa87::key_gen_internal(&self.private_key.into());

        if &mutation.public_key_hash != self.id() {
            anyhow::bail!(
                "received mutation for wrong cloud id: {}",
                self.metadata.name
            );
        }
        let salt: &[u8; 32] = &mutation.salt;

        mutation.verify(signer.verifying_key().encode().to_vec())?;

        let mutation_key_preimage = Bytes::encode(&(&self.private_key, mutation.index, &salt))?;
        let mutation_key = blake3::hash(&mutation_key_preimage.as_slice())
            .as_bytes()
            .to_vec();

        let mut tx_bytes = mutation.data; // encrypted tx data
        let mut chacha = ChaCha20::new(
            mutation_key.as_slice().into(),
            // we can safely choose 0 as the nonce because the encryption key is salted with a
            // strong random value preventing any encryption key from being used twice.
            vec![0_u8; 12].as_slice().into(),
        );
        chacha.apply_keystream(&mut tx_bytes);

        // tx_bytes is now decrypted
        Ok(Bytes::parse(&tx_bytes.into())?)
    }

    /// Accept an anondb transaction and create a trustless representation.
    fn encrypt_tx(&self, transaction: JournalTransaction) -> Result<Mutation> {
        let mut signer = MlDsa87::key_gen_internal(&self.private_key.into());

        let salt: [u8; 32] = rand::random();

        let mutation_key_preimage = Bytes::encode(&(&self.private_key, transaction.index, &salt))?;
        let mutation_key = blake3::hash(&mutation_key_preimage.as_slice())
            .as_bytes()
            .to_vec();

        // now we can encrypt the transaction data

        let mut tx_bytes: Vec<u8> = Bytes::encode(&transaction)?.into();
        let mut chacha = ChaCha20::new(
            mutation_key.as_slice().into(),
            // we can safely choose 0 as the nonce because the encryption key is salted with a
            // strong random value preventing any encryption key from being used twice.
            vec![0_u8; 12].as_slice().into(),
        );
        chacha.apply_keystream(&mut tx_bytes);
        drop(chacha);

        // tx_bytes are now encrypted

        let signature = signer.sign(&tx_bytes).encode().to_vec();

        Ok(Mutation {
            index: transaction.index,
            data: tx_bytes,
            signature,
            public_key_hash: self.id,
            public_key: if transaction.index == 0 {
                Some(self.public_key.clone())
            } else {
                None
            },
            salt,
            mutation_key: None,
        })
    }
}
