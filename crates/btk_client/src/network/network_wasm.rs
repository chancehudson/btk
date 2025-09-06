use anyhow::Result;
use futures_util::SinkExt;
use futures_util::StreamExt;
use gloo_net::websocket::Message;
use gloo_net::websocket::futures::WebSocket;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;

use network_common::*;

pub struct NetworkConnection {
    url: String,
    send_tx: flume::Sender<Action>,
    receive_rx: flume::Receiver<Response>,
    close_rx: flume::Receiver<()>,
    connected_rx: flume::Receiver<Result<()>>,
}

impl NetworkConnection {
    pub fn is_closed(&self) -> bool {
        !self.close_rx.is_empty()
    }

    pub fn is_open(&self) -> Result<bool> {
        if self.connected_rx.is_empty() {
            return Ok(false);
        }
        let msg = self.connected_rx.recv().unwrap();
        if let Err(e) = msg {
            Err(anyhow::format_err!(e))
        } else {
            Ok(true)
        }
    }

    pub fn attempt_connection(url: String) -> Self {
        let url_clone = url.clone();
        let (send_tx, send_rx) = flume::unbounded::<Action>();
        let (receive_tx, receive_rx) = flume::unbounded::<Response>();
        let (close_tx, close_rx) = flume::bounded::<()>(2);
        let (connected_tx, connected_rx) = flume::unbounded::<Result<()>>();
        // we'll start a thread to open the connection. If the connection succeeds we'll spawn one
        // thread for the read loop, and keep the genesis thread for the write loop
        spawn_local(async move {
            let mut ws = WebSocket::open(&url_clone);

            if let Err(e) = ws {
                web_sys::console::log_1(&"Connection errored".into());
                web_sys::console::log_2(&"err:".into(), &e.to_string().into());
                connected_tx.send(Err(anyhow::format_err!(e))).ok();
                return; // thread ends
            }

            let mut ws = ws.unwrap();
            let (mut write, mut read) = ws.split();

            web_sys::console::log_1(&"Connection succeeded".into());
            if let Err(_) = connected_tx.send(Ok(())) {
                println!("WARNING: No receivers for network connection attempt!");
                println!("halting connection thread");
                close_tx.send(()).ok();
                return; // thread ends
            }

            let close_tx_clone = close_tx.clone();
            spawn_local(async move {
                while let Some(msg) = read.next().await {
                    if msg.is_err() {
                        close_tx_clone.send(()).ok();
                        break;
                    }
                    let msg = msg.unwrap();
                    match msg {
                        Message::Bytes(bytes) => {
                            if let Ok(r) = Bytes::parse::<Response>(&bytes) {
                                if let Err(e) = receive_tx.send(r) {
                                    println!("receive err {:?}", e);
                                    break;
                                }
                            } else {
                                println!("failed to deserialize response");
                            }
                        }
                        _ => {
                            println!("received non-binary message, discarding");
                        }
                    }
                }
                println!("websocket read thread exiting");
            });

            loop {
                while let Ok(action) = send_rx.try_recv() {
                    if let Ok(serialized) = Bytes::encode(&action) {
                        if let Err(e) = write.send(Message::Bytes(serialized.into_vec())).await {
                            println!("Error sending ws message {:?}, closing connection", e);
                            close_tx.send(()).ok();
                            break;
                        }
                    }
                }
                TimeoutFuture::new(50).await;
            }
        });
        Self {
            url,
            send_tx,
            receive_rx,
            connected_rx,
            close_rx,
        }
    }

    /// Retrieve all pending messages from the server.
    pub fn read_connection(&self) -> Vec<Response> {
        self.receive_rx.drain().collect()
    }

    pub fn write_connection(&self, action: Action) {
        if let Err(e) = self.send_tx.send(action) {
            println!("error writing to network connection (wasm): {:?}", e);
        }
    }
}
