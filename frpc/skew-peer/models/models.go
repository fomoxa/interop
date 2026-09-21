package models

//fomoxa:model codec=rpc
type FRpcVoid struct{}

//fomoxa:model codec=rpc
type FRpcError struct {
	Code    uint32 `fomoxa:"u32" codec:"rpc"`
	Message string `fomoxa:"string" codec:"rpc"`
}

//fomoxa:model codec=rpc
type EchoRequest struct {
	Text string `fomoxa:"string" codec:"rpc"`
	Loud bool   `fomoxa:"bool" codec:"rpc"`
}

//fomoxa:model codec=rpc
type EchoResponse struct {
	Text string `fomoxa:"string" codec:"rpc"`
}
