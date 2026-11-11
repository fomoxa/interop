macro_rules! mount_the_generated_tree {
    () => {
        mod generated;
    };
}

mount_the_generated_tree!();
mod models;

use std::io::{Read, Write as _};
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_rpc::net::fomoxa_net::schema::{MessageSchema, Schema};
use fomoxa_rpc::net::fomoxa_net::session::{Config as NetConfig, SessionState};
use fomoxa_rpc::net::{RpcConnection, WaitPolicy};
use fomoxa_rpc::transport::BlockingTcp;
use fomoxa_rpc::{
    CallOptions, Config, Immediate, Method, Outcome, Registry, RegistryBuilder, Status,
    ThreadedServer,
};

use generated::{
    EchoRequestRpcCodec, EchoResponseRpcCodec, Reader, Writer, ECHO_REQUEST_RPC_MESSAGE_ID,
    ECHO_RESPONSE_RPC_MESSAGE_ID, FOMOXA_MESSAGES, FOMOXA_SCHEMA_FINGERPRINT,
    FRPC_ERROR_RPC_MESSAGE_ID, FRPC_VOID_RPC_MESSAGE_ID,
};
use models::echo::{EchoRequest, EchoResponse};

fn schema() -> Arc<Schema> {
    let messages = FOMOXA_MESSAGES
        .iter()
        .map(|message| MessageSchema::new(message.id, message.fingerprint, message.prefixes))
        .collect::<Vec<_>>();
    Arc::new(
        Schema::new(FOMOXA_SCHEMA_FINGERPRINT, messages)
            .expect("fomoxac never writes a schema this rejects"),
    )
}

fn say() -> Method {
    Method::unary(
        "Echo.Say",
        ECHO_REQUEST_RPC_MESSAGE_ID,
        ECHO_RESPONSE_RPC_MESSAGE_ID,
    )
}

fn builder() -> RegistryBuilder {
    Registry::builder(FRPC_VOID_RPC_MESSAGE_ID, FRPC_ERROR_RPC_MESSAGE_ID)
}

fn encode_request(text: &str, loud: bool) -> Vec<u8> {
    let mut writer = Writer::new();
    EchoRequestRpcCodec::encode(
        &mut writer,
        &EchoRequest {
            text: text.to_owned(),
            loud,
        },
    );
    writer.into_bytes()
}

fn encode_response(text: &str) -> Vec<u8> {
    let mut writer = Writer::new();
    EchoResponseRpcCodec::encode(
        &mut writer,
        &EchoResponse {
            text: text.to_owned(),
        },
    );
    writer.into_bytes()
}

fn decode_response(body: &[u8]) -> Result<String, String> {
    let mut value = EchoResponse::default();
    EchoResponseRpcCodec::decode(&mut Reader::new(body), &mut value)
        .map_err(|error| format!("{error:?}"))?;
    Ok(value.text)
}

fn serve(address: &str) -> ExitCode {
    let registry = builder()
        .serve(say(), |_, body| {
            let mut value = EchoRequest::default();
            EchoRequestRpcCodec::decode(&mut Reader::new(body), &mut value)
                .map_err(|error| Status::invalid_argument(format!("{error:?}")))?;
            let mut reply = value.text.to_uppercase();
            if value.loud {
                reply.push('!');
            }
            Ok(Immediate::respond(encode_response(&reply)))
        })
        .build()
        .expect("skew registry");
    let server = match ThreadedServer::bind(
        address,
        schema(),
        Arc::new(registry),
        NetConfig::default(),
        Config::default(),
    ) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("cannot bind {address}: {error}");
            return ExitCode::from(69);
        }
    };
    let handle = server.handle();
    println!("listening {}", server.local_addr());
    let _ = std::io::stdout().flush();
    let runner = thread::spawn(move || server.run());
    let mut rest = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut rest);
    handle.stop();
    let _ = runner.join();
    ExitCode::SUCCESS
}

fn ready(address: &str) -> Result<RpcConnection<BlockingTcp>, String> {
    let transport = BlockingTcp::connect(address).map_err(|error| error.to_string())?;
    let registry = builder()
        .declare(say())
        .build()
        .map_err(|error| error.to_string())?;
    let mut connection = RpcConnection::new(
        transport,
        schema(),
        Arc::new(registry),
        NetConfig::default(),
        Config::default(),
    );
    let give_up = Instant::now() + Duration::from_secs(5);
    loop {
        connection.tick(Instant::now());
        if connection.is_ready() {
            return Ok(connection);
        }
        if let Some(reason) = connection.handshake_failure() {
            return Err(format!("the handshake failed: {reason:?}"));
        }
        if connection.state() == SessionState::Closed || Instant::now() >= give_up {
            return Err("the session never became ready".to_owned());
        }
        let _ = connection.wait(WaitPolicy::default(), Instant::now());
    }
}

fn ask(address: &str, text: &str) -> Result<String, String> {
    let mut connection = ready(address)?;
    let call = connection
        .call(
            ECHO_REQUEST_RPC_MESSAGE_ID,
            &encode_request(text, true),
            None,
            CallOptions::default(),
            Instant::now(),
        )
        .map_err(|error| error.to_string())?;
    loop {
        connection.tick(Instant::now());
        if let Some(outcome) = connection
            .outcomes()
            .into_iter()
            .find(|outcome| outcome.call() == call)
        {
            return match outcome {
                Outcome::Response { body, .. } => decode_response(&body),
                other => Err(format!("{other:?}")),
            };
        }
        if connection.core().pending_request_id(call).is_none() {
            return Err(format!("{call} has no further outcome"));
        }
        let _ = connection.wait(WaitPolicy::default(), Instant::now());
    }
}

fn say_once(address: &str, text: &str) -> ExitCode {
    match ask(address, text) {
        Ok(reply) => {
            println!("{reply}");
            ExitCode::SUCCESS
        }
        Err(problem) => {
            eprintln!("{problem}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, address] if command == "serve" => serve(address),
        [command, address, text] if command == "say" => say_once(address, text),
        _ => {
            eprintln!("usage: frpc-skew-peer serve <host:port> | say <host:port> <text>");
            ExitCode::from(64)
        }
    }
}
