mod generated;
mod models;

use std::env;
use std::process::exit;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_net::connection::Connection;
use fomoxa_net::event::Event;
use fomoxa_net::schema::{MessageSchema, Schema};
use fomoxa_net::server::Server;
use fomoxa_net::session::Config;
use fomoxa_net::transport::{
    ServerTransport, TcpListenerTransport, TcpTransport, Transport, UdpServerTransport,
    UdpTransport,
};

use generated::{
    PlayerEdgeCodec, Reader, Writer, FOMOXA_MESSAGES, FOMOXA_SCHEMA_FINGERPRINT,
    PLAYER_EDGE_MESSAGE_ID,
};
use models::player::Player;

const TIMEOUT: Duration = Duration::from_secs(10);

const SERVER_SENDS: Player = Player { id: 100, x: 10.5, y: 20.0 };
const CLIENT_SENDS: Player = Player { id: 200, x: 1.0, y: 2.0 };

fn schema() -> Arc<Schema> {
    Arc::new(
        Schema::new(
            FOMOXA_SCHEMA_FINGERPRINT,
            FOMOXA_MESSAGES
                .iter()
                .map(|message| MessageSchema::new(message.id, message.fingerprint, message.prefixes))
                .collect::<Vec<_>>(),
        )
        .expect("fomoxac never writes a schema this rejects"),
    )
}

fn encode(player: &Player) -> Vec<u8> {
    let mut writer = Writer::new();
    PlayerEdgeCodec::encode(&mut writer, player);
    writer.into_bytes()
}

fn decode(bytes: &[u8]) -> Player {
    let mut value = Player::default();
    let mut reader = Reader::new(bytes);
    PlayerEdgeCodec::decode(&mut reader, &mut value).expect("peer sent an undecodable Player.edge");
    value
}

struct Args {
    role: String,
    addr: String,
    transport: String,
}

fn parse_args() -> Args {
    let mut role = None;
    let mut addr = None;
    let mut transport = "tcp".to_owned();
    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--role" => role = it.next(),
            "--addr" => addr = it.next(),
            "--transport" => {
                transport = it.next().unwrap_or_else(|| {
                    eprintln!("rust-peer: --transport needs a value");
                    exit(2);
                });
            }
            other => {
                eprintln!("rust-peer: unknown argument: {other}");
                exit(2);
            }
        }
    }
    let role = role.unwrap_or_else(|| {
        eprintln!("rust-peer: --role server|client is required");
        exit(2);
    });
    let addr = addr.unwrap_or_else(|| {
        eprintln!("rust-peer: --addr host:port is required");
        exit(2);
    });
    Args { role, addr, transport }
}

fn run_server<L: ServerTransport>(addr: &str, mut server: Server<L>) {
    println!("rust-peer: server listening on {addr}");

    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        let mut ready_peer = None;
        let mut received = None;
        for seen in server.tick_now() {
            match seen.event {
                Event::Ready => {
                    println!("rust-peer: peer {} ready, sending Player.edge", seen.peer);
                    ready_peer = Some(seen.peer);
                }
                Event::Message { id: PLAYER_EDGE_MESSAGE_ID, payload } => {
                    received = Some(decode(payload));
                }
                Event::HandshakeFailed(reason) => {
                    eprintln!("rust-peer: server handshake refused: {reason}");
                    exit(1);
                }
                _ => {}
            }
        }

        if let Some(peer) = ready_peer {
            let _ = server.send(peer, PLAYER_EDGE_MESSAGE_ID, &encode(&SERVER_SENDS));
        }

        if let Some(player) = received {
            if player != CLIENT_SENDS {
                eprintln!("rust-peer: server got unexpected value {player:?}");
                exit(1);
            }
            println!("rust-peer: server OK, received {player:?}");
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }

    eprintln!("rust-peer: server timed out waiting for the client");
    exit(1);
}

fn run_client<T: Transport>(addr: &str, mut connection: Connection<T>) {
    println!("rust-peer: client connected to {addr}");

    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        let mut received = None;
        for event in connection.tick_now() {
            match event {
                Event::Message { id: PLAYER_EDGE_MESSAGE_ID, payload } => {
                    received = Some(decode(payload));
                }
                Event::HandshakeFailed(reason) => {
                    eprintln!("rust-peer: client handshake refused: {reason}");
                    exit(1);
                }
                _ => {}
            }
        }

        if let Some(player) = received {
            if player != SERVER_SENDS {
                eprintln!("rust-peer: client got unexpected value {player:?}");
                exit(1);
            }
            println!("rust-peer: client OK, received {player:?}, replying");
            let _ = connection.send(PLAYER_EDGE_MESSAGE_ID, &encode(&CLIENT_SENDS));
            thread::sleep(Duration::from_millis(100));
            for _ in connection.tick_now() {}
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }

    eprintln!("rust-peer: client timed out waiting for the server");
    exit(1);
}

fn main() {
    let args = parse_args();
    match (args.role.as_str(), args.transport.as_str()) {
        ("server", "tcp") => {
            let listener = TcpListenerTransport::bind(&args.addr).expect("bind the interop address");
            run_server(&args.addr, Server::new(listener, schema(), Config::default()));
        }
        ("server", "udp") => {
            let listener = UdpServerTransport::bind(&args.addr).expect("bind the interop address");
            run_server(&args.addr, Server::new(listener, schema(), Config::default()));
        }
        ("client", "tcp") => {
            let transport = TcpTransport::connect(&args.addr).expect("connect to the interop address");
            run_client(&args.addr, Connection::new(transport, schema(), Config::default()));
        }
        ("client", "udp") => {
            let transport = UdpTransport::connect(&args.addr).expect("connect to the interop address");
            run_client(&args.addr, Connection::new(transport, schema(), Config::default()));
        }
        (role, transport) => {
            eprintln!(
                "rust-peer: unsupported combination: --role {role} --transport {transport} \
                 (role is server|client, transport is tcp|udp)"
            );
            exit(2);
        }
    }
    println!("rust-peer: done");
}
