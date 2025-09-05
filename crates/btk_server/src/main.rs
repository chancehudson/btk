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
    });

    let server = Arc::new(server::BTKServer::new().await?);

    // WebSocket core loop
    // start the websocket server loop in it's own thread
    let server_clone = server.clone();
    let shutdown_rx_clone = shutdown_rx.clone();
    println!("Starting websocket server");
    tokio::spawn(async move {
        while let Ok((stream, _)) = server_clone.network_server.listener.accept().await {
            // start ignoring tcp connections as shutdown is initiated
            if !shutdown_rx_clone.is_empty() {
                break;
            }
            let server_clone = server_clone.clone();
            tokio::spawn(async move {
                server_clone.network_server.accept_connection(stream).await;
            });
        }
    });

    // continuously handle client events as they are received
    println!("Listening for websocket actions");
    loop {
        if !shutdown_rx.is_empty() {
            break;
        }
        // handle inputs from the clients
        for (socket_id, action) in server.network_server.pending_actions.1.drain() {
            if let Err(e) = server.handle_action(socket_id, action.clone()).await {
                println!("failed to handle action: {:?} {:?}", action, e);
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    println!("Goodbye!");
    Ok(())
}
