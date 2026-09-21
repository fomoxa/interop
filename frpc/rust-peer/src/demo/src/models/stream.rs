#[allow(unused_imports)]
use fomoxa_attributes::{codec, network};

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct UploadOpen {
    #[network(string)]
    #[codec(rpc)]
    pub name: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct UploadChunk {
    #[network(u32)]
    #[codec(rpc)]
    pub size: u32,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct UploadDone {
    #[network(string)]
    #[codec(rpc)]
    pub summary: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct ChatOpen {
    #[network(string)]
    #[codec(rpc)]
    pub room: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct ChatSaid {
    #[network(string)]
    #[codec(rpc)]
    pub line: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct ChatHeard {
    #[network(string)]
    #[codec(rpc)]
    pub line: String,
}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct TicksRequest {}

#[derive(Debug, Default, Clone)]
#[network]
#[codec(rpc)]
pub struct TicksItem {
    #[network(u32)]
    #[codec(rpc)]
    pub value: u32,
}
