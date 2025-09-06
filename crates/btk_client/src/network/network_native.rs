use anondb::Bytes;
use anyhow::Result;
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use network_common::*;

pub struct NetworkConnection {
    url: String,
    send_tx: flume::Sender<Action>,
    receive_rx: flume::Receiver<Response>,
    connected_rx: flume::Receiver<Result<()>>,
    connected_tx: flume::Sender<Result<()>>,
    worker_thread: std::thread::JoinHandle<()>,
}

impl NetworkConnection {
    pub fn is_closed(&self) -> bool {
        self.worker_thread.is_finished()
    }

    pub fn is_open(&self) -> Result<bool> {
        if self.connected_rx.is_empty() {
            return Ok(false);
        }
        // this consumes the thing
        let msg = self.connected_rx.recv();
        if msg.is_err() {
            println!("WARNING: NetworkConnection: all senders are dropped");
        }
        let msg = msg.unwrap();
        // put the message back in the channel
        if let Err(e) = msg {
            // safe to unwrap because we're holding self.connected_rx
            self.connected_tx
                .send(Err(anyhow::anyhow!("original error consumed!")))
                .unwrap();
            Err(anyhow::format_err!(e))
        } else {
            self.connected_tx.send(Ok(())).unwrap();
            Ok(true)
        }
    }

    pub fn attempt_connection(url: String) -> Self {
        let url_clone = url.clone();
        let (send_tx, send_rx) = flume::unbounded::<Action>();
        let (receive_tx, receive_rx) = flume::unbounded::<Response>();
        let (connected_tx, connected_rx) = flume::unbounded::<Result<()>>();
        Self {
            url,
            connected_rx,
            connected_tx: connected_tx.clone(),
            send_tx,
            receive_rx,
            worker_thread: std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let connection = connect_async(url_clone).await;
                    if let Err(e) = connection {
                        println!("Connection errored: {:?}", e);
                        connected_tx.send(Err(anyhow::format_err!(e))).ok();
                        return; // thread ends
                    }
                    if let Ok((ws_stream, _)) = connection {
                        if let Err(_) = connected_tx.send(Ok(())) {
                            println!("WARNING: No receivers for network connection attempt!");
                            println!("halting connection thread");
                            return; // thread ends
                        }
                        let (mut write, mut read) = ws_stream.split();
                        tokio::spawn(async move {
                            while let Some(Ok(msg)) = read.next().await {
                                if msg.is_binary() {
                                    if let Ok(r) =
                                        Bytes::parse::<Response>(&msg.into_data().to_vec().into())
                                    {
                                        if let Err(e) = receive_tx.send(r) {
                                            println!("receive err {:?}", e);
                                            break;
                                        }
                                    } else {
                                        println!("failed to deserialize response");
                                    }
                                } else {
                                    println!("non-binary message");
                                }
                            }
                        });
                        while let Ok(action) = send_rx.recv_async().await {
                            if let Ok(serialized) = Bytes::encode(&action) {
                                if let Err(e) =
                                    write.send(Message::binary(serialized.into_vec())).await
                                {
                                    println!("error sending {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                    // thread ends
                });
            }),
        }
    }

    /// Retrieve all pending messages from the server.
    pub fn read_connection(&self) -> Vec<Response> {
        self.receive_rx.drain().collect()
    }

    pub fn write_connection(&self, action: Action) {
        if let Err(e) = self.send_tx.send(action) {
            println!("error writing to network connection (native): {:?}", e);
        }
    }
}
