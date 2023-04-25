//! Based on tokio-tungstenite example websocket client, but with multiple
//! concurrent websocket clients in one package
//!
//! This will connect to a server specified in the SERVER with N_CLIENTS
//! concurrent connections, and then flood some test messages over websocket.
//! This will also print whatever it gets into stdout.
//!
//! Note that this is not currently optimized for performance, especially around
//! stdout mutex management. Rather it's intended to show an example of working with axum's
//! websocket server and how the client-side and server-side code can be quite similar.
//!

use futures_util::{SinkExt, StreamExt};
use std::ops::ControlFlow;
use std::time::Instant;

// we will use tungstenite for websocket client impl (same library as what axum is using)
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const SERVER: &str = "ws://127.0.0.1:3000/ws";

#[tokio::main]
async fn main() {
    let ws_stream = match connect_async(SERVER).await {
        Ok((stream, response)) => {
            println!("Handshake for client has been completed");
            // This will be the HTTP response, same as with server this is the last moment we
            // can still access HTTP stuff.
            println!("Server response was {:?}", response);
            stream
        }
        Err(e) => {
            println!("WebSocket handshake for client failed with {e}!");
            return;
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();

    //spawn an async sender to push some more messages into the server
    for i in 1..3 {
        // In any websocket error, break loop.
        if sender
            .send(Message::Binary(
                format!("Message number {}...", i).as_bytes().to_vec(),
            ))
            .await
            .is_err()
        {
            //just as with server, if send fails there is nothing we can do but exit.
            return;
        }

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    //receiver just prints whatever it gets
    let mut count = 0;
    let start = Instant::now();
    while let Some(Ok(msg)) = receiver.next().await {
        count += 1;
        let who = format!("{count} {:?}", start.elapsed());
        // print message and break if instructed to do so
        if process_message(msg, who).is_break() {
            break;
        }
    }
}

/// Function to handle messages we get (with a slight twist that Frame variant is visible
/// since we are working with the underlying tungstenite library directly without axum here).
fn process_message(msg: Message, who: String) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {} got str: {:?}", who, t);
        }
        Message::Binary(d) => {
            println!(">>> {} got {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} got close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {} somehow got close message without CloseFrame", who);
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {} got pong with {:?}", who, v);
        }
        // Just as with axum server, the underlying tungstenite websocket library
        // will handle Ping for you automagically by replying with Pong and copying the
        // v according to spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {} got ping with {:?}", who, v);
        }

        Message::Frame(_) => {
            unreachable!("This is never supposed to happen")
        }
    }
    ControlFlow::Continue(())
}
