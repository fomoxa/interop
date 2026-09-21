#[allow(unused_imports)]
use fomoxa_attributes::{codec, network};

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct EchoRequest {
    #[network(string)]
    #[codec(rpc)]
    pub text: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct EchoResponse {
    #[network(string)]
    #[codec(rpc)]
    pub text: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct CountRequest {
    #[network(u32)]
    #[codec(rpc)]
    pub count: u32,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct CountItem {
    #[network(u32)]
    #[codec(rpc)]
    pub value: u32,
}
