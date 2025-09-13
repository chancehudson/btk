use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;

mod network;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    // shutdown channel
    let (shutdown_tx, shutdown_rx) = flume::bounded::<()>(1);
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => println!("Received SIGTERM"),
            _ = sigint.recv() => println!("Received SIGINT"),
            _ = tokio::signal::ctrl_c() => println!("Received Ctrl+C"),
        }
        shutdown_tx.send(()).unwrap();
        println!("Goodbye!");
    });

    let server = Arc::new(server::BTKServer::new().await?);

    // WebSocket core loop
    // start the websocket server loop in it's own thread
    {
        let server_clone = server.clone();
        let shutdown_rx_clone = shutdown_rx.clone();
        tokio::spawn(async move {
            println!("Starting websocket server");
            while let Ok((stream, _)) = server_clone.network_server.listener.accept().await {
                if shutdown_rx_clone.is_full() {
                    break;
                }
                let server_clone = server_clone.clone();
                tokio::spawn(async move {
                    server_clone.network_server.accept_connection(stream).await;
                });
            }
        });
    }

    // http core loop
    {
        let server_clone = server.clone();
        let shutdown_rx_clone = shutdown_rx.clone();
        tokio::spawn(async move {
            println!("Starting http server");
            let http_server =
                tiny_http::Server::http("0.0.0.0:8000").expect("failed to start http server");
            loop {
                if shutdown_rx_clone.is_full() {
                    break;
                }
                match http_server.recv_timeout(Duration::from_secs(1)) {
                    Ok(req) => {
                        if req.is_none() {
                            continue;
                        }
                        let req = req.unwrap();
                        let server_clone = server_clone.clone();
                        tokio::spawn(async move {
                            if let Err(e) = server_clone.handle_req(req).await {
                                println!("error handling http req: {:?}", e);
                            }
                        });
                        tokio::task::yield_now().await;
                    }
                    Err(e) => {
                        println!("http server errored: {:?}", e);
                        break;
                    }
                }
            }
        });
    }

    // run the final task on the main thread
    {
        let server_clone = server.clone();
        let shutdown_rx_clone = shutdown_rx.clone();
        // continuously handle client events as they are received
        println!("Listening for websocket actions");
        loop {
            if shutdown_rx_clone.is_full() {
                break;
            }
            // handle inputs from the clients
            match server_clone
                .network_server
                .pending_actions
                .1
                .recv_timeout(Duration::from_secs(1))
            {
                Ok((socket_id, action)) => {
                    if let Err(e) = server.handle_action(socket_id, action.clone()).await {
                        println!("failed to handle action: {:?} {:?}", action, e);
                    }
                }
                Err(e) => {
                    if matches!(e, flume::RecvTimeoutError::Timeout) {
                        continue;
                    } else {
                        panic!("no senders for pending_actions channel");
                    }
                }
            }
        }
    }

    Ok(())
}
