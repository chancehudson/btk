use std::path::PathBuf;

use anondb::Bytes;
use anondb::Journal;
use anondb::TransactionOperation;
use anyhow::Result;
use chacha20::ChaCha20;
use chacha20::cipher::KeyIvInit;
use chacha20::cipher::StreamCipher;
use ml_dsa::KeyGen;
use ml_dsa::MlDsa87;
use ml_dsa::signature::SignerMut;

use network_common::Mutation;

use super::remote_cloud::RemoteCloud;

/// Meta info about an encrypted cloud.
pub struct Cloud {
    /// Public key that identifies the cloud.
    public_key: Vec<u8>,
    /// Private key that may mutate the cloud.
    private_key: [u8; 32],
    /// Latest known mutation index.
    latest_known_index: u64,
    pub db: Journal,
    id: [u8; 32],
    remote: Option<RemoteCloud>,
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

    pub fn new(data_dir_maybe: Option<PathBuf>) -> Result<Self> {
        Self::from_key(rand::random(), data_dir_maybe)
    }

    pub fn from_key(private_key: [u8; 32], data_dir_maybe: Option<PathBuf>) -> Result<Self> {
        let signer = MlDsa87::key_gen_internal(&private_key.into());
        let public_key = signer.verifying_key().encode().to_vec();
        let id: [u8; 32] = blake3::hash(&public_key).into();
        let hex_string = id.iter().map(|b| format!("{:02x}", b)).collect::<String>() + ".redb";

        Ok(Self {
            id,
            private_key,
            public_key,
            latest_known_index: 0,
            db: if let Some(data_dir) = data_dir_maybe {
                Journal::at_path(&data_dir.join(hex_string))?
            } else {
                Journal::in_memory(None)?
            },
            remote: None,
        })
    }

    /// Accept an anondb transaction and create a trustless representation.
    fn encrypt_tx(&self, index: u64, transaction: Vec<TransactionOperation>) -> Result<Mutation> {
        let mut signer = MlDsa87::key_gen_internal(&self.private_key.into());

        let salt: [u8; 32] = rand::random();

        let mutation_key_preimage = Bytes::encode(&(&self.private_key, index, &salt))?;
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
            public_key_hash: self.id,
            public_key: if index == 0 {
                Some(self.public_key.clone())
            } else {
                None
            },
            salt,
            mutation_key: None,
        })
    }
}
