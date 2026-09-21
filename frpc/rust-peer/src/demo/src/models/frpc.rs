#[allow(unused_imports)]
use fomoxa_attributes::{codec, network};

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcVoid {}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcError {
    #[network(u32)]
    #[codec(rpc)]
    pub code: u32,
    #[network(string)]
    #[codec(rpc)]
    pub message: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcCredit {
    #[network(u32)]
    #[codec(rpc)]
    pub items: u32,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcCtx {
    #[network(u32)]
    #[codec(rpc)]
    pub deadline_ms: u32,
    #[network(bytes)]
    #[codec(rpc)]
    pub trace_id: Vec<u8>,
    #[network(bytes)]
    #[codec(rpc)]
    pub span_id: Vec<u8>,
    #[network(string)]
    #[codec(rpc)]
    pub token: String,
    #[network(string)]
    #[codec(rpc)]
    pub tenant: String,
    #[network(bytes)]
    #[codec(rpc)]
    pub idempotency_key: Vec<u8>,
    #[network(u32)]
    #[codec(rpc)]
    pub initial_credit: u32,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcReflectRequest {}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcMethodInfo {
    #[network(string)]
    #[codec(rpc)]
    pub name: String,
    #[network(u32)]
    #[codec(rpc)]
    pub shape: u32,
    #[network(u32)]
    #[codec(rpc)]
    pub request_id: u32,
    #[network(u32)]
    #[codec(rpc)]
    pub caller_item_id: u32,
    #[network(u32)]
    #[codec(rpc)]
    pub reply_id: u32,
    #[network(u32)]
    #[codec(rpc)]
    pub retry_safety: u32,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct FRpcReflectResponse {
    #[network(Array<FRpcMethodInfo>)]
    #[codec(rpc)]
    pub methods: Vec<FRpcMethodInfo>,
    #[network(bytes)]
    #[codec(rpc)]
    pub schema_json: Vec<u8>,
}
