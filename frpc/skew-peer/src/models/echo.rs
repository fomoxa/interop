#[allow(unused_imports)]
use fomoxa_attributes::{codec, network};

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct EchoRequest {
    #[network(string)]
    #[codec(rpc)]
    pub text: String,
    #[network(bool)]
    #[codec(rpc)]
    pub loud: bool,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct EchoResponse {
    #[network(string)]
    #[codec(rpc)]
    pub text: String,
}
