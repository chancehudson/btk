use anondb::Bytes;
use anondb::Journal;
use anyhow::Result;

use network_common::*;

use super::network;

const PUBLIC_KEY_TABLE: &str = "known_public_keys";

pub struct BTKServer {
    pub db: Journal,
    pub network_server: network::Server,
}

impl BTKServer {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            // db: redb::Database::create("./data.redb")?.into(),
            db: Journal::in_memory(None)?,
            network_server: network::Server::new().await?,
        })
    }

    pub async fn handle_action(&self, socket_id: String, action: Action) -> Result<()> {
        match action {
            Action::Ping => {
                // TODO: flood prevention
                self.network_server.send(&socket_id, Response::Pong).await?;
            }
            Action::MutateCloud(mutation) => {
                let table_name = hex::encode(mutation.public_key_hash);
                let public_key = if let Some(public_key) = &mutation.public_key {
                    public_key.clone()
                } else if let Some(public_key) = self
                    .db
                    .get::<[u8; 32], Bytes>(&table_name, &mutation.public_key_hash)?
                {
                    public_key.to_vec()
                } else {
                    anyhow::bail!("unknown public key for cloud id: {}", table_name);
                };
                mutation.verify(public_key.clone())?;

                let mut tx = self.db.begin_write()?;
                let mut table = tx.open_table(&table_name)?;
                let existing_mutation_count = table.len()?;

                if mutation.index != existing_mutation_count {
                    anyhow::bail!(
                        "bad mutation index, expected {} got {}",
                        existing_mutation_count,
                        mutation.index
                    );
                }

                if mutation.index == 0 {
                    let mut pubkey_table = tx.open_table(&PUBLIC_KEY_TABLE)?;
                    pubkey_table
                        .insert::<[u8; 32], Bytes>(&mutation.public_key_hash, &public_key.into())?;
                }
                table.insert(&mutation.index, &mutation)?;
                drop(table);

                tx.commit()?;

                // TODO: broadcast the new mutation
            }
            Action::AuthCloud(cloud_id, sig_bytes) => {
                unimplemented!();
            }
            Action::GetMutation(cloud_id, mutation_index) => {
                let table_name = hex::encode(cloud_id);
                if let Some(mutation) = self.db.get(&table_name, &mutation_index)? {
                    self.network_server
                        .send(&socket_id, Response::CloudMutation(mutation))
                        .await?;
                }
            }
        }
        Ok(())
    }
}
