use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::RwLock;

use anondb::Bytes;
use anondb::Journal;
use network_common::Mutation;
use worker::*;

const MUTATION_COUNT_TABLE_NAME: &str = "mutation_counts";

fn mutation_key(cloud_id: &[u8; 32], index: u32) -> String {
    format!("cloud-{}-{}", hex::encode(cloud_id), index)
}

fn mutation_count_key(cloud_id: &[u8; 32]) -> String {
    format!("cloud-{}-count", hex::encode(cloud_id))
}

#[durable_object]
pub struct StorageCoordinator {
    /// cloud id keyed to number of mutations
    db: Journal,
    mutation_count_lock: RwLock<Option<u32>>,
    authed_listeners: RwLock<Vec<WebSocket>>,
    state: State,
    env: Env,
}

impl StorageCoordinator {
    pub async fn get_public_key(&self, hash: [u8; 32]) -> Result<Option<Vec<u8>>> {
        let bucket = self.env.bucket("btk_storage")?;
        let obj = bucket.get(hex::encode(hash)).execute().await?;
        if obj.is_none() {
            return Ok(None);
        }
        let obj = obj.unwrap();
        if obj.body().is_none() {
            return Ok(None);
        }
        let body = obj.body().unwrap();
        Ok(Some(body.bytes().await?))
    }

    pub async fn load_mutation_count(&self, cloud_id: &[u8; 32]) -> Result<u32> {
        let bucket = self.env.bucket("btk_storage")?;
        let count = bucket.get(mutation_count_key(cloud_id)).execute().await?;
        if count.is_none() {
            return Ok(0);
        }
        let count = count.unwrap();
        let body = count.body();
        if body.is_none() {
            return Ok(0);
        }
        let body = body.unwrap();
        let count = Bytes::parse::<u32>(&body.bytes().await?.into())
            .map_err(|_| "failed to parse count body")?;
        Ok(count)
    }

    pub async fn mutation_count(&self, cloud_id: &[u8; 32]) -> Result<u32> {
        if let Some(count) = *self.mutation_count_lock.read().unwrap() {
            return Ok(count);
        }
        let mut mutation_count_maybe = self.mutation_count_lock.write().unwrap();
        let mutation_count = self.load_mutation_count(cloud_id).await?;
        *mutation_count_maybe = Some(mutation_count);
        Ok(mutation_count)
    }
}

impl DurableObject for StorageCoordinator {
    fn new(state: State, env: Env) -> Self {
        Self {
            db: Journal::in_memory(None).expect("failed to init anondb"),
            mutation_count_lock: RwLock::new(None),
            state,
            env,
            authed_listeners: RwLock::new(Vec::default()),
        }
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        self.authed_listeners.write().unwrap().retain(|v| v != &ws);
        Ok(())
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        match message {
            WebSocketIncomingMessage::String(_) => {
                ws.send_with_str("pong")?;
            }
            WebSocketIncomingMessage::Binary(_bytes) => {}
        }
        Ok(())
    }

    async fn fetch(&self, mut req: Request) -> worker::Result<Response> {
        let upgrade_header = req.headers().get("Upgrade")?.unwrap_or_default();
        if upgrade_header == "websocket" {
            let ws = WebSocketPair::new()?;
            let client = ws.client;
            let server = ws.server;
            server.accept()?;
            self.authed_listeners.write().unwrap().push(server);

            return worker::Response::from_websocket(client);
        }

        let bucket = self.env.bucket("btk_storage")?;

        let headers = Headers::new();
        headers.set("Access-Control-Allow-Origin", "*")?;

        // build the url query into a usable format
        let mut query: HashMap<String, String> = HashMap::default();
        for (key, val) in req.url()?.query_pairs() {
            query.insert(key.to_string(), val.to_string());
        }
        match (req.method(), req.path().as_str()) {
            (Method::Get, "/") => Response::ok("hello"),
            (Method::Get, "/state") => {
                let cloud_id = if let Some(cloud_id_str) = query.get("cloud_id") {
                    let mut out = [0u8; 32];
                    match hex::decode_to_slice(cloud_id_str.to_string(), &mut out) {
                        Ok(_) => out,
                        Err(_) => {
                            return Ok(Response::empty()?.with_status(400));
                        }
                    }
                } else {
                    return Ok(Response::empty()?.with_status(400));
                };
                let count = self.mutation_count(&cloud_id).await?;
                Ok(Response::from_bytes(
                    Bytes::encode(&(count as u64))
                        .map_err(|_| "encoding failed")?
                        .into(),
                )?
                .with_headers(headers))
            }
            (Method::Get, "/mutation") => {
                let cloud_id = if let Some(cloud_id_str) = query.get("cloud_id") {
                    let mut out = [0u8; 32];
                    match hex::decode_to_slice(cloud_id_str.to_string(), &mut out) {
                        Ok(_) => out,
                        Err(_) => {
                            return Ok(Response::empty()?.with_status(400));
                        }
                    }
                } else {
                    return Ok(Response::empty()?.with_status(400));
                };

                let index = if let Some(index_str) = query.get("index") {
                    u32::from_str_radix(&index_str.to_string(), 10)
                        .map_err(|_| "failed to parse index")?
                } else {
                    return Ok(Response::empty()?.with_status(400));
                };
                let mutation_count = self.mutation_count(&cloud_id).await?;
                if index >= mutation_count {
                    return Ok(Response::empty()?.with_status(424));
                }
                let obj_maybe = bucket.get(mutation_key(&cloud_id, index)).execute().await?;
                Ok(
                    Response::from_body(obj_maybe.unwrap().body().unwrap().response_body()?)?
                        .with_headers(headers),
                )
            }
            (Method::Post, "/mutate") => {
                let mutation_bytes = req.bytes().await?;
                let mutation = Bytes::from(&mutation_bytes)
                    .parse::<Mutation>()
                    .map_err(|_| "failed to parse body")?;
                let public_key = if let Some(public_key) = &mutation.public_key {
                    public_key.clone()
                } else if let Some(public_key) =
                    self.get_public_key(mutation.public_key_hash).await?
                {
                    public_key
                } else {
                    return Ok(Response::empty()?.with_status(400));
                };
                if let Err(e) = mutation.verify(public_key.clone()) {
                    println!("error verifying mutation: {:?}", e);
                    return Ok(Response::empty()?.with_status(401));
                }
                let cloud_id = mutation.public_key_hash;

                let mut mutation_count_maybe = self.mutation_count_lock.write().unwrap();
                let mutation_count = self.load_mutation_count(&cloud_id).await?;

                if mutation.index != mutation_count as u64 {
                    return Ok(Response::empty()?.with_status(400));
                }
                let new_mutation_count = mutation.index + 1;

                if mutation.index == 0 {
                    bucket
                        .put(hex::encode(mutation.public_key_hash), public_key)
                        .execute()
                        .await?;
                }

                bucket
                    .put(
                        mutation_key(&mutation.public_key_hash, mutation.index as u32),
                        // req.inner().body().unwrap(),
                        mutation_bytes,
                    )
                    .execute()
                    .await?;
                bucket
                    .put(
                        mutation_count_key(&cloud_id),
                        Bytes::encode(&new_mutation_count)
                            .map_err(|_| "failed to encode new count")?
                            .to_vec(),
                    )
                    .execute()
                    .await?;
                *mutation_count_maybe = Some(new_mutation_count as u32);
                drop(mutation_count_maybe);

                match Bytes::encode(&network_common::Response::CloudMutated(new_mutation_count)) {
                    Ok(bytes) => {
                        for ws in self.authed_listeners.read().unwrap().iter() {
                            ws.send_with_bytes(bytes.clone()).ok();
                        }
                    }
                    Err(e) => {
                        println!("error sending to ws: {e:?}");
                    }
                }

                Ok(Response::empty()?.with_status(204).with_headers(headers))
            }
            _ => Ok(Response::empty()?.with_status(404)),
        }
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    // build the url query into a usable format
    let mut query: HashMap<String, String> = HashMap::default();
    for (key, val) in req.url()?.query_pairs() {
        query.insert(key.to_string(), val.to_string());
    }
    let cloud_id = if let Some(cloud_id_str) = query.get("cloud_id") {
        if cloud_id_str.len() != 64 {
            return Ok(Response::empty()?.with_status(400));
        }
        cloud_id_str
    } else {
        return Ok(Response::empty()?.with_status(400));
    };
    let namespace = env.durable_object("BTK_PRERELEASE")?;
    let stub = namespace.id_from_name(cloud_id)?.get_stub()?;
    stub.fetch_with_request(req).await
}
