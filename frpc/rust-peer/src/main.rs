#[path = "demo/src/models/mod.rs"]
mod models;

macro_rules! mount_the_generated_tree {
    () => {
        #[path = "demo/src/generated/mod.rs"]
        mod generated;
    };
}

mount_the_generated_tree!();

#[path = "demo/src/netproto.rs"]
mod netproto;

use std::collections::VecDeque;
use std::io::{Read, Write as _};
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_rpc::net::fomoxa_net::session::{Config as NetConfig, SessionState};
use fomoxa_rpc::net::{RpcConnection, WaitPolicy};
use fomoxa_rpc::queue::Delivery;
use fomoxa_rpc::transport::BlockingTcp;
use fomoxa_rpc::{
    reflect, status, CallError, CallId, CallOptions, Config, Ctx, Handler, Immediate, Method,
    Outcome, Progress, Registry, RegistryBuilder, Role, RpcCore, Status, ThreadedServer,
};

use generated::{
    ChatHeardRpcCodec, ChatOpenRpcCodec, ChatSaidRpcCodec, CountItemRpcCodec, CountRequestRpcCodec,
    EchoRequestRpcCodec, EchoResponseRpcCodec, FRpcReflectRequestRpcCodec,
    FRpcReflectResponseRpcCodec, Reader, TicksItemRpcCodec, TicksRequestRpcCodec,
    UploadChunkRpcCodec, UploadDoneRpcCodec, UploadOpenRpcCodec, Writer, CHAT_HEARD_RPC_MESSAGE_ID,
    CHAT_OPEN_RPC_MESSAGE_ID, CHAT_SAID_RPC_MESSAGE_ID, COUNT_ITEM_RPC_MESSAGE_ID,
    COUNT_REQUEST_RPC_MESSAGE_ID, ECHO_REQUEST_RPC_MESSAGE_ID, ECHO_RESPONSE_RPC_MESSAGE_ID,
    FOMOXA_SCHEMA_FINGERPRINT, FRPC_CREDIT_RPC_MESSAGE_ID, FRPC_CTX_RPC_MESSAGE_ID,
    FRPC_ERROR_RPC_MESSAGE_ID, FRPC_REFLECT_REQUEST_RPC_MESSAGE_ID,
    FRPC_REFLECT_RESPONSE_RPC_MESSAGE_ID, FRPC_VOID_RPC_MESSAGE_ID, TICKS_ITEM_RPC_MESSAGE_ID,
    TICKS_REQUEST_RPC_MESSAGE_ID, UPLOAD_CHUNK_RPC_MESSAGE_ID, UPLOAD_DONE_RPC_MESSAGE_ID,
    UPLOAD_OPEN_RPC_MESSAGE_ID,
};
use models::echo::{CountItem, CountRequest, EchoRequest, EchoResponse};
use models::frpc::{FRpcReflectRequest, FRpcReflectResponse};
use models::stream::{
    ChatHeard, ChatOpen, ChatSaid, TicksItem, TicksRequest, UploadChunk, UploadDone, UploadOpen,
};
use netproto::schema;

const SCHEMA_JSON: &str = include_str!("demo/.fomoxa/schema.json");

fn say() -> Method {
    Method::unary("Echo.Say", ECHO_REQUEST_RPC_MESSAGE_ID, ECHO_RESPONSE_RPC_MESSAGE_ID)
}

fn count() -> Method {
    Method::server_stream("Echo.Count", COUNT_REQUEST_RPC_MESSAGE_ID, COUNT_ITEM_RPC_MESSAGE_ID)
}

fn upload() -> Method {
    Method::client_stream(
        "Blob.Upload",
        UPLOAD_OPEN_RPC_MESSAGE_ID,
        UPLOAD_CHUNK_RPC_MESSAGE_ID,
        UPLOAD_DONE_RPC_MESSAGE_ID,
    )
}

fn chat() -> Method {
    Method::bidi_stream(
        "Room.Chat",
        CHAT_OPEN_RPC_MESSAGE_ID,
        CHAT_SAID_RPC_MESSAGE_ID,
        CHAT_HEARD_RPC_MESSAGE_ID,
    )
}

fn ticks() -> Method {
    Method::server_stream("Clock.Ticks", TICKS_REQUEST_RPC_MESSAGE_ID, TICKS_ITEM_RPC_MESSAGE_ID)
}

fn reflect_method() -> Method {
    Method::unary(
        reflect::METHOD_NAME,
        FRPC_REFLECT_REQUEST_RPC_MESSAGE_ID,
        FRPC_REFLECT_RESPONSE_RPC_MESSAGE_ID,
    )
}

fn missing() -> Method {
    Method::unary("Echo.Missing", FRPC_CTX_RPC_MESSAGE_ID, ECHO_RESPONSE_RPC_MESSAGE_ID)
}

fn builder() -> RegistryBuilder {
    Registry::builder(FRPC_VOID_RPC_MESSAGE_ID, FRPC_ERROR_RPC_MESSAGE_ID)
        .credit(FRPC_CREDIT_RPC_MESSAGE_ID)
}

fn driver_registry() -> Arc<Registry> {
    let builder = builder()
        .declare(reflect_method())
        .declare(say())
        .declare(count())
        .declare(upload())
        .declare(chat())
        .declare(ticks())
        .declare(missing());
    Arc::new(builder.build().expect("driver registry"))
}

fn responder_registry(reflection: bool) -> Arc<Registry> {
    let mut builder = builder()
        .serve(say(), |_, body| {
            let text = decode_echo(body);
            if text.is_empty() {
                return Err(Status::invalid_argument("Echo.Say needs a non-empty text"));
            }
            Ok(Immediate::respond(encode_echo_response(&text.to_uppercase())))
        })
        .serve(count(), |_, body| {
            let count = decode_count(body);
            if count > 100 {
                return Err(Status::invalid_argument("Echo.Count is capped at 100"));
            }
            Ok(Box::new(Counter { next: 0, count }))
        })
        .serve(upload(), |_, _| Ok(Box::new(Upload { bytes: 0, chunks: 0, closed: false })))
        .serve(chat(), |_, _| Ok(Box::new(Chat { replies: VecDeque::new(), closed: false })))
        .serve(ticks(), |_, _| Ok(Box::new(Ticks { next: 0 })));
    if reflection {
        builder = builder.reflection(
            FRPC_REFLECT_REQUEST_RPC_MESSAGE_ID,
            FRPC_REFLECT_RESPONSE_RPC_MESSAGE_ID,
            SCHEMA_JSON,
        );
    }
    Arc::new(builder.build().expect("responder registry"))
}

fn encode(write: impl FnOnce(&mut Writer)) -> Vec<u8> {
    let mut writer = Writer::new();
    write(&mut writer);
    writer.into_bytes()
}

fn encode_echo(text: &str) -> Vec<u8> {
    encode(|writer| EchoRequestRpcCodec::encode(writer, &EchoRequest { text: text.to_owned() }))
}

fn decode_echo(bytes: &[u8]) -> String {
    let mut value = EchoRequest::default();
    let _ = EchoRequestRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.text
}

fn encode_echo_response(text: &str) -> Vec<u8> {
    encode(|writer| EchoResponseRpcCodec::encode(writer, &EchoResponse { text: text.to_owned() }))
}

fn decode_echo_response(bytes: &[u8]) -> String {
    let mut value = EchoResponse::default();
    let _ = EchoResponseRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.text
}

fn encode_count(count: u32) -> Vec<u8> {
    encode(|writer| CountRequestRpcCodec::encode(writer, &CountRequest { count }))
}

fn decode_count(bytes: &[u8]) -> u32 {
    let mut value = CountRequest::default();
    let _ = CountRequestRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.count
}

fn encode_count_item(value: u32) -> Vec<u8> {
    encode(|writer| CountItemRpcCodec::encode(writer, &CountItem { value }))
}

fn decode_count_item(bytes: &[u8]) -> u32 {
    let mut value = CountItem::default();
    let _ = CountItemRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.value
}

fn encode_upload_open(name: &str) -> Vec<u8> {
    encode(|writer| UploadOpenRpcCodec::encode(writer, &UploadOpen { name: name.to_owned() }))
}

fn encode_chunk(size: u32) -> Vec<u8> {
    encode(|writer| UploadChunkRpcCodec::encode(writer, &UploadChunk { size }))
}

fn decode_chunk(bytes: &[u8]) -> u32 {
    let mut value = UploadChunk::default();
    let _ = UploadChunkRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.size
}

fn encode_done(summary: &str) -> Vec<u8> {
    encode(|writer| UploadDoneRpcCodec::encode(writer, &UploadDone { summary: summary.to_owned() }))
}

fn decode_done(bytes: &[u8]) -> String {
    let mut value = UploadDone::default();
    let _ = UploadDoneRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.summary
}

fn encode_room(room: &str) -> Vec<u8> {
    encode(|writer| ChatOpenRpcCodec::encode(writer, &ChatOpen { room: room.to_owned() }))
}

fn encode_said(line: &str) -> Vec<u8> {
    encode(|writer| ChatSaidRpcCodec::encode(writer, &ChatSaid { line: line.to_owned() }))
}

fn decode_said(bytes: &[u8]) -> String {
    let mut value = ChatSaid::default();
    let _ = ChatSaidRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.line
}

fn encode_heard(line: &str) -> Vec<u8> {
    encode(|writer| ChatHeardRpcCodec::encode(writer, &ChatHeard { line: line.to_owned() }))
}

fn decode_heard(bytes: &[u8]) -> String {
    let mut value = ChatHeard::default();
    let _ = ChatHeardRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.line
}

fn encode_ticks_request() -> Vec<u8> {
    encode(|writer| TicksRequestRpcCodec::encode(writer, &TicksRequest {}))
}

fn encode_tick(value: u32) -> Vec<u8> {
    encode(|writer| TicksItemRpcCodec::encode(writer, &TicksItem { value }))
}

fn decode_tick(bytes: &[u8]) -> u32 {
    let mut value = TicksItem::default();
    let _ = TicksItemRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value.value
}

fn encode_reflect_request() -> Vec<u8> {
    encode(|writer| FRpcReflectRequestRpcCodec::encode(writer, &FRpcReflectRequest {}))
}

fn decode_reflect_response(bytes: &[u8]) -> FRpcReflectResponse {
    let mut value = FRpcReflectResponse::default();
    let _ = FRpcReflectResponseRpcCodec::decode(&mut Reader::new(bytes), &mut value);
    value
}

struct Counter {
    next: u32,
    count: u32,
}

impl Handler for Counter {
    fn poll(&mut self, _now: Instant) -> Progress {
        if self.next >= self.count {
            return Progress::End;
        }
        self.next += 1;
        Progress::Item(encode_count_item(self.next))
    }
}

struct Upload {
    bytes: u32,
    chunks: u32,
    closed: bool,
}

impl Handler for Upload {
    fn poll(&mut self, _now: Instant) -> Progress {
        if !self.closed {
            return Progress::Pending;
        }
        Progress::Respond(encode_done(&format!("{} chunks, {} bytes", self.chunks, self.bytes)))
    }

    fn item(&mut self, body: &[u8]) -> Result<(), Status> {
        let size = decode_chunk(body);
        if size == 0 {
            return Err(Status::invalid_argument("an empty chunk is not accepted"));
        }
        self.chunks += 1;
        self.bytes += size;
        Ok(())
    }

    fn end_of_stream(&mut self) {
        self.closed = true;
    }
}

struct Chat {
    replies: VecDeque<Vec<u8>>,
    closed: bool,
}

impl Handler for Chat {
    fn poll(&mut self, _now: Instant) -> Progress {
        if let Some(reply) = self.replies.pop_front() {
            return Progress::Item(reply);
        }
        if self.closed {
            return Progress::End;
        }
        Progress::Pending
    }

    fn item(&mut self, body: &[u8]) -> Result<(), Status> {
        self.replies.push_back(encode_heard(&decode_said(body).to_uppercase()));
        Ok(())
    }

    fn end_of_stream(&mut self) {
        self.closed = true;
    }
}

struct Ticks {
    next: u32,
}

impl Handler for Ticks {
    fn poll(&mut self, _now: Instant) -> Progress {
        self.next += 1;
        Progress::Item(encode_tick(self.next))
    }
}

struct Client {
    connection: RpcConnection<BlockingTcp>,
    policy: WaitPolicy,
    backlog: Vec<Outcome>,
}

impl Client {
    fn open(address: &str) -> Result<Client, String> {
        let transport = BlockingTcp::connect(address).map_err(|error| error.to_string())?;
        let connection = RpcConnection::new(
            transport,
            schema(),
            driver_registry(),
            NetConfig::default(),
            Config::default(),
        );
        let mut client = Client { connection, policy: WaitPolicy::default(), backlog: Vec::new() };
        let give_up = Instant::now() + Duration::from_secs(5);
        loop {
            client.pump();
            if client.connection.is_ready() {
                return Ok(client);
            }
            if let Some(reason) = client.connection.handshake_failure() {
                return Err(format!("the handshake failed: {reason:?}"));
            }
            if client.connection.state() == SessionState::Closed || Instant::now() >= give_up {
                return Err("the session never became ready".to_owned());
            }
            let _ = client.connection.wait(client.policy, Instant::now());
        }
    }

    fn pump(&mut self) {
        self.connection.tick(Instant::now());
        self.backlog.extend(self.connection.outcomes());
    }

    fn call(&mut self, request_id: u32, body: &[u8], ctx: Option<&Ctx>) -> Result<CallId, String> {
        self.call_within(request_id, body, ctx, CallOptions::default())
    }

    fn call_within(
        &mut self,
        request_id: u32,
        body: &[u8],
        ctx: Option<&Ctx>,
        options: CallOptions,
    ) -> Result<CallId, String> {
        let call = self
            .connection
            .call(request_id, body, ctx, options, Instant::now())
            .map_err(|error| error.to_string())?;
        self.pump();
        Ok(call)
    }

    fn next(&mut self, call: CallId) -> Result<Outcome, String> {
        loop {
            if let Some(index) = self.backlog.iter().position(|outcome| outcome.call() == call) {
                return Ok(self.backlog.remove(index));
            }
            if self.connection.core().pending_request_id(call).is_none() {
                return Err(format!("{call} has no further outcome"));
            }
            let _ = self.connection.wait(self.policy, Instant::now());
            self.pump();
        }
    }

    fn invoke(&mut self, request_id: u32, body: &[u8]) -> Result<Outcome, String> {
        let call = self.call(request_id, body, None)?;
        self.next(call)
    }

    fn send_item(&mut self, call: CallId, body: &[u8]) -> Result<(), String> {
        loop {
            match self.connection.core_mut().send_item(call, body) {
                Err(CallError::NoCredit) | Err(CallError::Congested) => {
                    if self.connection.state() == SessionState::Closed {
                        return Err("the session closed".to_owned());
                    }
                    let _ = self.connection.wait(self.policy, Instant::now());
                    self.pump();
                }
                other => {
                    self.pump();
                    return other.map_err(|error| error.to_string());
                }
            }
        }
    }

    fn close_send(&mut self, call: CallId) -> Result<(), String> {
        let closed = self.connection.core_mut().close_send(call);
        self.pump();
        closed.map_err(|error| error.to_string())
    }

    fn grant(&mut self, call: CallId, items: u32) -> Result<(), String> {
        let granted = self.connection.core_mut().grant(call, items);
        self.pump();
        granted.map_err(|error| error.to_string())
    }

    fn cancel(&mut self, call: CallId) {
        self.connection.cancel(call);
        self.pump();
    }

    fn next_tick(&mut self, call: CallId) -> Result<u32, String> {
        match self.next(call)? {
            Outcome::Item { body, .. } => Ok(decode_tick(&body)),
            other => Err(format!("expected a tick, got {other:?}")),
        }
    }
}

fn expect_status(outcome: Outcome, code: u32) -> Result<(), String> {
    match outcome {
        Outcome::Failed { status, .. } if status.code == code => Ok(()),
        other => Err(format!("expected {}, got {other:?}", status::name_of(code))),
    }
}

fn say_uppercases(client: &mut Client) -> Result<(), String> {
    match client.invoke(ECHO_REQUEST_RPC_MESSAGE_ID, &encode_echo("xin chao"))? {
        Outcome::Response { body, .. } if decode_echo_response(&body) == "XIN CHAO" => Ok(()),
        other => Err(format!("{other:?}")),
    }
}

fn say_rejects_empty(client: &mut Client) -> Result<(), String> {
    let outcome = client.invoke(ECHO_REQUEST_RPC_MESSAGE_ID, &encode_echo(""))?;
    expect_status(outcome, status::INVALID_ARGUMENT)
}

fn count_streams(client: &mut Client) -> Result<(), String> {
    let call = client.call(COUNT_REQUEST_RPC_MESSAGE_ID, &encode_count(5), None)?;
    let mut seen = Vec::new();
    loop {
        match client.next(call)? {
            Outcome::Item { body, .. } => seen.push(decode_count_item(&body)),
            Outcome::End { .. } if seen == [1, 2, 3, 4, 5] => return Ok(()),
            other => return Err(format!("{other:?} after {seen:?}")),
        }
    }
}

fn count_rejects_large(client: &mut Client) -> Result<(), String> {
    let call = client.call(COUNT_REQUEST_RPC_MESSAGE_ID, &encode_count(101), None)?;
    expect_status(client.next(call)?, status::INVALID_ARGUMENT)
}

fn upload_summarises(client: &mut Client) -> Result<(), String> {
    let call = client.call(UPLOAD_OPEN_RPC_MESSAGE_ID, &encode_upload_open("photo.raw"), None)?;
    for size in [4_096u32, 8_192, 1_024] {
        client.send_item(call, &encode_chunk(size))?;
    }
    client.close_send(call)?;
    match client.next(call)? {
        Outcome::Response { body, .. } if decode_done(&body) == "3 chunks, 13312 bytes" => Ok(()),
        other => Err(format!("{other:?}")),
    }
}

fn upload_rejects_empty_chunk(client: &mut Client) -> Result<(), String> {
    let call = client.call(UPLOAD_OPEN_RPC_MESSAGE_ID, &encode_upload_open("empty.raw"), None)?;
    client.send_item(call, &encode_chunk(0))?;
    expect_status(client.next(call)?, status::INVALID_ARGUMENT)
}

fn chat_echoes(client: &mut Client) -> Result<(), String> {
    let call = client.call(CHAT_OPEN_RPC_MESSAGE_ID, &encode_room("lobby"), None)?;
    for line in ["hello", "still here", "bye"] {
        client.send_item(call, &encode_said(line))?;
        match client.next(call)? {
            Outcome::Item { body, .. } if decode_heard(&body) == line.to_uppercase() => {}
            other => return Err(format!("{other:?} for {line}")),
        }
    }
    client.close_send(call)?;
    match client.next(call)? {
        Outcome::End { .. } => Ok(()),
        other => Err(format!("{other:?}")),
    }
}

fn ticks_follow_credit(client: &mut Client) -> Result<(), String> {
    let ctx = Ctx::with_initial_credit(2);
    let call = client.call(TICKS_REQUEST_RPC_MESSAGE_ID, &encode_ticks_request(), Some(&ctx))?;
    let mut seen = vec![client.next_tick(call)?, client.next_tick(call)?];
    let quiet_until = Instant::now() + Duration::from_millis(150);
    while Instant::now() < quiet_until {
        let _ = client.connection.wait(client.policy, Instant::now());
        client.pump();
    }
    if client.connection.core().pending_request_id(call).is_none() {
        return Err("the stream ended while the allowance was zero".to_owned());
    }
    client.grant(call, 3)?;
    for _ in 0..3 {
        seen.push(client.next_tick(call)?);
    }
    client.cancel(call);
    expect_status(client.next(call)?, status::CANCELLED)?;
    if seen == [1, 2, 3, 4, 5] {
        Ok(())
    } else {
        Err(format!("items {seen:?}"))
    }
}

fn ticks_expire(client: &mut Client) -> Result<(), String> {
    let ctx = Ctx::with_initial_credit(1);
    let call = client.call_within(
        TICKS_REQUEST_RPC_MESSAGE_ID,
        &encode_ticks_request(),
        Some(&ctx),
        CallOptions::within(Duration::from_millis(200)),
    )?;
    client.next_tick(call)?;
    expect_status(client.next(call)?, status::DEADLINE_EXCEEDED)
}

fn missing_is_unimplemented(client: &mut Client) -> Result<(), String> {
    let outcome = client.invoke(FRPC_CTX_RPC_MESSAGE_ID, &[])?;
    expect_status(outcome, status::UNIMPLEMENTED)
}

fn describe(name: &str, shape: u32, ids: [u32; 3], retry: u32) -> String {
    format!("{name}/{shape}/{:08X}/{:08X}/{:08X}/{retry}", ids[0], ids[1], ids[2])
}

fn reflection_lists(client: &mut Client) -> Result<(), String> {
    let body =
        match client.invoke(FRPC_REFLECT_REQUEST_RPC_MESSAGE_ID, &encode_reflect_request())? {
            Outcome::Response { body, .. } => body,
            other => return Err(format!("{other:?}")),
        };
    let answer = decode_reflect_response(&body);
    let mut expected: Vec<String> =
        [say(), count(), upload(), chat(), ticks(), reflect_method().idempotent()]
            .iter()
            .map(|method| {
                describe(
                    &method.name,
                    reflect::shape_code(method.shape),
                    [method.request_id, method.caller_item_id(), method.reply_id],
                    reflect::retry_code(method.retry_safety),
                )
            })
            .collect();
    let mut listed: Vec<String> = answer
        .methods
        .iter()
        .map(|info| {
            describe(
                &info.name,
                info.shape,
                [info.request_id, info.caller_item_id, info.reply_id],
                info.retry_safety,
            )
        })
        .collect();
    expected.sort();
    listed.sort();
    if expected != listed {
        return Err(format!("listed {listed:?}"));
    }
    let schema_json = String::from_utf8_lossy(&answer.schema_json);
    let wanted = format!("\"fingerprint_u64\": \"0x{FOMOXA_SCHEMA_FINGERPRINT:016X}\"");
    if schema_json.contains(&wanted) {
        Ok(())
    } else {
        Err("schema_json does not carry this schema's fingerprint".to_owned())
    }
}

type Check = fn(&mut Client) -> Result<(), String>;

fn drive(address: &str) -> ExitCode {
    let mut client = match Client::open(address) {
        Ok(client) => client,
        Err(problem) => {
            eprintln!("{problem}");
            return ExitCode::from(69);
        }
    };
    let checks: [(&str, Check); 11] = [
        ("unary Echo.Say", say_uppercases),
        ("unary error Echo.Say empty", say_rejects_empty),
        ("server stream Echo.Count", count_streams),
        ("server stream error Echo.Count 101", count_rejects_large),
        ("client stream Blob.Upload", upload_summarises),
        ("client stream error Blob.Upload empty chunk", upload_rejects_empty_chunk),
        ("bidi Room.Chat", chat_echoes),
        ("credit Clock.Ticks", ticks_follow_credit),
        ("deadline Clock.Ticks", ticks_expire),
        ("unimplemented Echo.Missing", missing_is_unimplemented),
        ("reflection FRpc.Reflect", reflection_lists),
    ];
    let mut failures = 0;
    for (name, check) in checks {
        match check(&mut client) {
            Ok(()) => println!("ok   {name}"),
            Err(problem) => {
                failures += 1;
                println!("FAIL {name}: {problem}");
            }
        }
    }
    client.connection.close();
    if failures == 0 {
        println!("all checks passed");
        ExitCode::SUCCESS
    } else {
        println!("{failures} checks failed");
        ExitCode::FAILURE
    }
}

fn serve(address: &str) -> ExitCode {
    let server = match ThreadedServer::bind(
        address,
        schema(),
        responder_registry(true),
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

fn take(core: &mut RpcCore, side: &str, lines: &mut Vec<String>) -> Vec<(u32, Vec<u8>)> {
    let mut frames = Vec::new();
    core.drain(|id, payload| {
        let hex: String = payload.iter().map(|byte| format!("{byte:02X}")).collect();
        lines.push(format!("{side} {id:08X} {hex}"));
        frames.push((id, payload.to_vec()));
        Delivery::Accepted
    });
    frames
}

fn exchange(client: &mut RpcCore, server: &mut RpcCore, now: Instant, lines: &mut Vec<String>) {
    for (id, payload) in take(client, "C", lines) {
        server.on_message(id, &payload, now);
    }
    server.tick(now);
    for (id, payload) in take(server, "S", lines) {
        client.on_message(id, &payload, now);
    }
    client.tick(now);
    client.outcomes().for_each(drop);
}

fn frames() -> ExitCode {
    let now = Instant::now();
    let mut client = RpcCore::new(driver_registry(), Role::Client, Config::default());
    let mut server = RpcCore::new(responder_registry(false), Role::Server, Config::default());
    client.on_ready();
    server.on_ready();
    let open = CallOptions::without_deadline();

    let traced = Ctx {
        deadline_ms: 1500,
        trace_id: vec![1, 2, 3, 4],
        span_id: vec![5, 6],
        token: "tok".to_owned(),
        tenant: "acme".to_owned(),
        ..Ctx::default()
    };
    let steps = (|| -> Result<CallId, CallError> {
        client.call(
            ECHO_REQUEST_RPC_MESSAGE_ID,
            &encode_echo("xin chao"),
            Some(&traced),
            open,
            now,
        )?;
        let credit = Ctx::with_initial_credit(2);
        let ticks = client.call(
            TICKS_REQUEST_RPC_MESSAGE_ID,
            &encode_ticks_request(),
            Some(&credit),
            open,
            now,
        )?;
        client.grant(ticks, 3)?;
        let upload = client.call(
            UPLOAD_OPEN_RPC_MESSAGE_ID,
            &encode_upload_open("photo.raw"),
            None,
            open,
            now,
        )?;
        client.send_item(upload, &encode_chunk(4_096))?;
        client.send_item(upload, &encode_chunk(1_024))?;
        client.close_send(upload)?;
        client.call(ECHO_REQUEST_RPC_MESSAGE_ID, &encode_echo(""), None, open, now)?;
        client.call(FRPC_CTX_RPC_MESSAGE_ID, &[], None, open, now)?;
        let chat = client.call(CHAT_OPEN_RPC_MESSAGE_ID, &encode_room("lobby"), None, open, now)?;
        client.send_item(chat, &encode_said("hello"))?;
        client.close_send(chat)?;
        Ok(ticks)
    })();
    let ticks = match steps {
        Ok(ticks) => ticks,
        Err(error) => {
            eprintln!("the scenario could not be built: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut lines = Vec::new();
    exchange(&mut client, &mut server, now, &mut lines);
    client.cancel(ticks);
    exchange(&mut client, &mut server, now, &mut lines);
    for line in lines {
        println!("{line}");
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, address] if command == "serve" => serve(address),
        [command, address] if command == "drive" => drive(address),
        [command] if command == "frames" => frames(),
        [command, address, text] if command == "say" => say_once(address, text),
        _ => {
            eprintln!("usage: interop serve <host:port> | drive <host:port> | frames | say <host:port> <text>");
            ExitCode::from(64)
        }
    }
}

fn say_once(address: &str, text: &str) -> ExitCode {
    let answered = Client::open(address).and_then(|mut client| {
        match client.invoke(ECHO_REQUEST_RPC_MESSAGE_ID, &encode_echo(text))? {
            Outcome::Response { body, .. } => Ok(decode_echo_response(&body)),
            other => Err(format!("{other:?}")),
        }
    });
    match answered {
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
