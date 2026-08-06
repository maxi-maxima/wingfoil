use log::Level::Info;
use log::info;
use std::{net::TcpListener, thread::spawn};

use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
};

fn start_server() {
    //env_logger::init();
    let server = TcpListener::bind("127.0.0.1:3012").unwrap();
    for stream in server.incoming() {
        spawn(move || {
            let callback = |req: &Request, mut response: Response| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().path());
                println!("The request's headers are:");
                for (header, _value) in req.headers() {
                    println!("* {header}");
                }

                // Let's add an additional header to our response to the client.
                let headers = response.headers_mut();
                headers.append("MyCustomHeader", ":)".parse().unwrap());
                headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                Ok(response)
            };
            let mut websocket = accept_hdr(stream.unwrap(), callback).unwrap();

            loop {
                let msg = websocket.read().unwrap();
                if msg.is_binary() || msg.is_text() {
                    websocket.send(msg).unwrap();
                }
            }
        });
    }
}

#[test]
fn test_sub() {
    start_server()
}

// NOT FINISHED

// ```sh
// cargo test --features fix-integration-test -p wingfoil \
//   -- lmax --nocapture --test-threads=1
// ```

// 1. **`test_connection_refused`** — error propagates correctly
// 2. **`test_sub_snapshot`** — pre-seeded data appears in snapshot phase
// 3. **`test_sub_live_updates`** — events arrive after snapshot
// 4. **`test_pub_round_trip`** — `pub` writes → verify via direct client read
// 5. **`test_sub_no_race`** — concurrent write during snapshot→watch handoff not missed or duplicated (if applicable)
// 6. **`test_delete_events`** — delete/tombstone events handled correctly (if applicable)
