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
