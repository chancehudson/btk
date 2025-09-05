use anyhow::Result;

use network_common::*;

use super::network;

pub struct BTKServer {
    pub db: redb::Database,
    pub network_server: network::Server,
}

impl BTKServer {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            db: redb::Database::create("./data.redb")?,
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
                unimplemented!();
            }
            Action::AuthCloud(cloud_id, sig_bytes) => {
                unimplemented!();
            }
            Action::GetMutation(mutation_index) => {
                unimplemented!();
            }
        }
        Ok(())
    }
}
