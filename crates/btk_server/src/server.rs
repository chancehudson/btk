use std::collections::HashMap;
use std::str::FromStr;

use anondb::Bytes;
use anondb::Journal;
use anyhow::Result;

use network_common::*;
use serde::Serialize;
use tiny_http::Header;
use tiny_http::Method;
use tiny_http::Request;
use url::Url;

use super::network;

const PUBLIC_KEY_TABLE: &str = "known_public_keys";

pub struct Req {
    pub url: url::Url,
    pub path: String,
    pub query: HashMap<String, String>,
    pub body: Vec<u8>,
    pub method: tiny_http::Method,
    request: tiny_http::Request,
}

impl TryFrom<tiny_http::Request> for Req {
    type Error = anyhow::Error;

    fn try_from(mut value: tiny_http::Request) -> std::result::Result<Self, Self::Error> {
        let url = Url::parse(&format!("http://0.0.0.0{}", value.url()))?;
        let mut query = HashMap::default();
        for (key, val) in url.query_pairs() {
            query.insert(key.to_string(), val.to_string());
        }
        let mut body = Vec::default();
        value.as_reader().read_to_end(&mut body).unwrap();

        Ok(Self {
            method: value.method().clone(),
            path: url.path().to_string(),
            url,
            query,
            body,
            request: value,
        })
    }
}

impl Req {
    pub fn path_tuple(&self) -> (&Method, &str) {
        (&self.method, &self.path)
    }

    pub fn respond_empty(self, status: u32) -> Result<()> {
        self.respond::<()>(status, None)
    }

    pub fn respond<T>(self, status: u32, data_maybe: Option<T>) -> Result<()>
    where
        T: Serialize,
    {
        let data = if let Some(data) = data_maybe {
            let bytes = Bytes::encode(&data)?;
            bytes.to_vec()
        } else {
            vec![]
        };
        let response = tiny_http::Response::empty(status)
            .with_data(data.as_slice(), Some(data.len()))
            .with_header(Header::from_str("Access-Control-Allow-Origin:*").unwrap());
        self.request.respond(response)?;
        Ok(())
    }
}

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
    pub async fn handle_req(&self, req: Request) -> Result<()> {
        let req = Req::try_from(req)?;
        match req.path_tuple() {
            (Method::Get, "/state") => {
                let cloud_id = if let Some(cloud_id_str) = req.query.get("cloud_id") {
                    match hex::decode(cloud_id_str.to_string()) {
                        Ok(id) => id,
                        Err(_) => {
                            return req.respond_empty(400);
                        }
                    }
                } else {
                    return req.respond_empty(400);
                };
                if cloud_id.len() != 32 {
                    return req.respond_empty(400);
                }
                let table_name = hex::encode(cloud_id);
                let mutation_count = self.db.count::<Bytes, Bytes>(&table_name)?;
                req.respond(200, Some(mutation_count))
            }
            (Method::Get, "/mutation") => {
                // retrieve a mutation for a cloud by index
                let cloud_id = if let Some(cloud_id_str) = req.query.get("cloud_id") {
                    match hex::decode(cloud_id_str.to_string()) {
                        Ok(id) => id,
                        Err(_) => {
                            return req.respond_empty(400);
                        }
                    }
                } else {
                    return req.respond_empty(400);
                };
                let index = if let Some(index_str) = req.query.get("index") {
                    u64::from_str_radix(&index_str.to_string(), 10)?
                } else {
                    return req.respond_empty(400);
                };
                let table_name = hex::encode(cloud_id);
                if let Some(mutation) = self.db.get::<u64, Mutation>(&table_name, &index.into())? {
                    return req.respond(200, Some(mutation));
                } else {
                    return req.respond_empty(404);
                }
            }
            (Method::Post, "/mutate") => {
                println!("receiving mutation");
                let mutation = Bytes::from(&req.body).parse::<Mutation>()?;
                let table_name = hex::encode(mutation.public_key_hash);
                let public_key = if let Some(public_key) = &mutation.public_key {
                    public_key.clone()
                } else if let Some(public_key) = self
                    .db
                    .get::<[u8; 32], Bytes>(PUBLIC_KEY_TABLE, &mutation.public_key_hash)?
                {
                    public_key.to_vec()
                } else {
                    return req.respond_empty(400);
                };
                if let Err(e) = mutation.verify(public_key.clone()) {
                    println!("error verifying mutation: {:?}", e);
                    return req.respond_empty(401);
                }

                let mut tx = self.db.begin_write()?;
                let mut table = tx.open_table(&table_name)?;
                let existing_mutation_count = table.len()?;

                if mutation.index != existing_mutation_count {
                    return req.respond_empty(410);
                }

                if mutation.index == 0 {
                    let mut pubkey_table = tx.open_table(&PUBLIC_KEY_TABLE)?;
                    pubkey_table
                        .insert::<[u8; 32], Bytes>(&mutation.public_key_hash, &public_key.into())?;
                }
                table.insert(&mutation.index, &mutation)?;
                drop(table);

                tx.commit()?;

                req.respond_empty(204)

                // TODO: broadcast the new mutation
            }
            _ => req.respond_empty(410),
        }
    }

    /// Handle a websocket action
    pub async fn handle_action(&self, socket_id: String, action: Action) -> Result<()> {
        match action {
            Action::Ping => {
                // TODO: flood prevention
                self.network_server.send(&socket_id, Response::Pong).await?;
            }
            Action::MutateCloud(_mutation) => {}
            Action::AuthCloud(_cloud_id, _sig_bytes) => {
                unimplemented!();
            }
        }
        Ok(())
    }
}
