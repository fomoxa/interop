#![allow(dead_code)]

use std::sync::Arc;
use std::time::Instant;

use fomoxa_rpc::{Handler, Method, Progress, Registry, Status};

use crate::generated::{
    CountItemRpcCodec, CountRequestRpcCodec, EchoRequestRpcCodec, EchoResponseRpcCodec, Reader,
    Writer, COUNT_ITEM_RPC_MESSAGE_ID, COUNT_REQUEST_RPC_MESSAGE_ID, ECHO_REQUEST_RPC_MESSAGE_ID,
    ECHO_RESPONSE_RPC_MESSAGE_ID, FRPC_CREDIT_RPC_MESSAGE_ID, FRPC_ERROR_RPC_MESSAGE_ID,
    FRPC_REFLECT_REQUEST_RPC_MESSAGE_ID, FRPC_REFLECT_RESPONSE_RPC_MESSAGE_ID,
    FRPC_VOID_RPC_MESSAGE_ID,
};
use crate::models::echo::{CountItem, CountRequest, EchoRequest, EchoResponse};

pub const VOID_ID: u32 = FRPC_VOID_RPC_MESSAGE_ID;
pub const ERROR_ID: u32 = FRPC_ERROR_RPC_MESSAGE_ID;
pub const CREDIT_ID: u32 = FRPC_CREDIT_RPC_MESSAGE_ID;
pub const ECHO_REQUEST: u32 = ECHO_REQUEST_RPC_MESSAGE_ID;
pub const ECHO_RESPONSE: u32 = ECHO_RESPONSE_RPC_MESSAGE_ID;
pub const COUNT_REQUEST: u32 = COUNT_REQUEST_RPC_MESSAGE_ID;
pub const COUNT_ITEM: u32 = COUNT_ITEM_RPC_MESSAGE_ID;
pub const REFLECT_REQUEST: u32 = FRPC_REFLECT_REQUEST_RPC_MESSAGE_ID;
pub const REFLECT_RESPONSE: u32 = FRPC_REFLECT_RESPONSE_RPC_MESSAGE_ID;

pub const SCHEMA_JSON: &str = include_str!("../.fomoxa/schema.json");

pub fn encode_request(text: &str) -> Vec<u8> {
    let mut writer = Writer::new();
    EchoRequestRpcCodec::encode(&mut writer, &EchoRequest { text: text.to_owned() });
    writer.into_bytes()
}

pub fn decode_request(bytes: &[u8]) -> EchoRequest {
    let mut value = EchoRequest::default();
    let mut reader = Reader::new(bytes);
    let _ = EchoRequestRpcCodec::decode(&mut reader, &mut value);
    value
}

pub fn encode_response(text: &str) -> Vec<u8> {
    let mut writer = Writer::new();
    EchoResponseRpcCodec::encode(&mut writer, &EchoResponse { text: text.to_owned() });
    writer.into_bytes()
}

pub fn decode_response(bytes: &[u8]) -> EchoResponse {
    let mut value = EchoResponse::default();
    let mut reader = Reader::new(bytes);
    let _ = EchoResponseRpcCodec::decode(&mut reader, &mut value);
    value
}

pub fn encode_count(count: u32) -> Vec<u8> {
    let mut writer = Writer::new();
    CountRequestRpcCodec::encode(&mut writer, &CountRequest { count });
    writer.into_bytes()
}

pub fn decode_count(bytes: &[u8]) -> CountRequest {
    let mut value = CountRequest::default();
    let mut reader = Reader::new(bytes);
    let _ = CountRequestRpcCodec::decode(&mut reader, &mut value);
    value
}

pub fn encode_item(value: u32) -> Vec<u8> {
    let mut writer = Writer::new();
    CountItemRpcCodec::encode(&mut writer, &CountItem { value });
    writer.into_bytes()
}

pub fn decode_item(bytes: &[u8]) -> CountItem {
    let mut value = CountItem::default();
    let mut reader = Reader::new(bytes);
    let _ = CountItemRpcCodec::decode(&mut reader, &mut value);
    value
}

pub fn methods() -> [Method; 2] {
    [
        Method::unary("Echo.Say", ECHO_REQUEST, ECHO_RESPONSE),
        Method::server_stream("Echo.Count", COUNT_REQUEST, COUNT_ITEM),
    ]
}

fn builder() -> fomoxa_rpc::RegistryBuilder {
    Registry::builder(VOID_ID, ERROR_ID).credit(CREDIT_ID)
}

pub fn caller_registry() -> Arc<Registry> {
    let mut builder = builder().declare(Method::unary(
        fomoxa_rpc::reflect::METHOD_NAME,
        REFLECT_REQUEST,
        REFLECT_RESPONSE,
    ));
    for method in methods() {
        builder = builder.declare(method);
    }
    Arc::new(builder.build().expect("caller registry"))
}

pub fn responder_registry() -> Arc<Registry> {
    let [say, count] = methods();
    Arc::new(
        builder()
            .serve(say, |_, body| {
                let request = decode_request(body);
                if request.text.is_empty() {
                    return Err(Status::invalid_argument("Echo.Say needs a non-empty text"));
                }
                Ok(Box::new(Shout { text: Some(request.text.to_uppercase()) }))
            })
            .serve(count, |_, body| {
                let request = decode_count(body);
                if request.count > 100 {
                    return Err(Status::invalid_argument("Echo.Count is capped at 100"));
                }
                Ok(Box::new(Counter { next: 0, count: request.count }))
            })
            .reflection(REFLECT_REQUEST, REFLECT_RESPONSE, SCHEMA_JSON)
            .build()
            .expect("responder registry"),
    )
}

struct Shout {
    text: Option<String>,
}

impl Handler for Shout {
    fn poll(&mut self, _now: Instant) -> Progress {
        match self.text.take() {
            Some(text) => Progress::Respond(encode_response(&text)),
            None => Progress::Fail(Status::internal("polled after completion")),
        }
    }
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
        Progress::Item(encode_item(self.next))
    }
}
