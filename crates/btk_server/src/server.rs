use anondb::Bytes;
use anondb::Journal;
use anyhow::Result;

use network_common::*;
use tiny_http::Method;
use tiny_http::Request;
use tiny_http::StatusCode;

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

    /// Handle an http action
    pub async fn handle_req(&self, mut req: Request) -> Result<()> {
        match req.method() {
            Method::Get => {
                if req.url() == "/state" {
                    println!("{}", req.url());
                }
            }
            Method::Post => {
                if req.url() == "/mutate" {
                    let mut data = Vec::default();
                    req.as_reader().read_to_end(&mut data)?;
                    let mutation = Bytes::from(data).parse::<Mutation>()?;
                    let table_name = hex::encode(mutation.public_key_hash);
                    let public_key = if let Some(public_key) = &mutation.public_key {
                        public_key.clone()
                    } else if let Some(public_key) = self
                        .db
                        .get::<[u8; 32], Bytes>(&table_name, &mutation.public_key_hash)?
                    {
                        public_key.to_vec()
                    } else {
                        req.respond(tiny_http::Response::empty(400))?;
                        return Ok(());
                    };
                    if let Err(e) = mutation.verify(public_key.clone()) {
                        println!("error verifying mutation: {:?}", e);
                        req.respond(tiny_http::Response::empty(401))?;
                        return Ok(());
                    }

                    let mut tx = self.db.begin_write()?;
                    let mut table = tx.open_table(&table_name)?;
                    let existing_mutation_count = table.len()?;

                    if mutation.index != existing_mutation_count {
                        req.respond(tiny_http::Response::empty(410))?;
                        return Ok(());
                    }

                    if mutation.index == 0 {
                        let mut pubkey_table = tx.open_table(&PUBLIC_KEY_TABLE)?;
                        pubkey_table.insert::<[u8; 32], Bytes>(
                            &mutation.public_key_hash,
                            &public_key.into(),
                        )?;
                    }
                    table.insert(&mutation.index, &mutation)?;
                    drop(table);

                    tx.commit()?;

                    req.respond(tiny_http::Response::empty(204))?;

                    // TODO: broadcast the new mutation
                } else {
                    req.respond(tiny_http::Response::empty(404))?;
                }
            }
            _ => req.respond(tiny_http::Response::empty(400))?,
        }
        Ok(())
    }

    /// Handle a websocket action
    pub async fn handle_action(&self, socket_id: String, action: Action) -> Result<()> {
        match action {
            Action::Ping => {
                // TODO: flood prevention
                self.network_server.send(&socket_id, Response::Pong).await?;
            }
            Action::MutateCloud(mutation) => {}
            Action::AuthCloud(cloud_id, sig_bytes) => {
                unimplemented!();
            }
        }
        Ok(())
    }
}
