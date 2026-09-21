using System;
using System.Collections.Generic;
using Fomoxa;

namespace Fomoxa.Rpc.Demo.Models
{
    [Network]
    [Codec("rpc")]
    public class FRpcVoid
    {
    }

    [Network]
    [Codec("rpc")]
    public class FRpcError
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint Code { get; set; }

        [Network("string")]
        [Codec("rpc")]
        public string Message { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class FRpcCredit
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint Items { get; set; }
    }

    [Network]
    [Codec("rpc")]
    public class FRpcCtx
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint DeadlineMs { get; set; }

        [Network("bytes")]
        [Codec("rpc")]
        public byte[] TraceId { get; set; } = Array.Empty<byte>();

        [Network("bytes")]
        [Codec("rpc")]
        public byte[] SpanId { get; set; } = Array.Empty<byte>();

        [Network("string")]
        [Codec("rpc")]
        public string Token { get; set; } = "";

        [Network("string")]
        [Codec("rpc")]
        public string Tenant { get; set; } = "";

        [Network("bytes")]
        [Codec("rpc")]
        public byte[] IdempotencyKey { get; set; } = Array.Empty<byte>();

        [Network("u32")]
        [Codec("rpc")]
        public uint InitialCredit { get; set; }
    }

    [Network]
    [Codec("rpc")]
    public class FRpcReflectRequest
    {
    }

    [Network]
    [Codec("rpc")]
    public class FRpcMethodInfo
    {
        [Network("string")]
        [Codec("rpc")]
        public string Name { get; set; } = "";

        [Network("u32")]
        [Codec("rpc")]
        public uint Shape { get; set; }

        [Network("u32")]
        [Codec("rpc")]
        public uint RequestId { get; set; }

        [Network("u32")]
        [Codec("rpc")]
        public uint CallerItemId { get; set; }

        [Network("u32")]
        [Codec("rpc")]
        public uint ReplyId { get; set; }

        [Network("u32")]
        [Codec("rpc")]
        public uint RetrySafety { get; set; }
    }

    [Network]
    [Codec("rpc")]
    public class FRpcReflectResponse
    {
        [Network("Array<FRpcMethodInfo>")]
        [Codec("rpc")]
        public List<FRpcMethodInfo> Methods { get; set; } = new List<FRpcMethodInfo>();

        [Network("bytes")]
        [Codec("rpc")]
        public byte[] SchemaJson { get; set; } = Array.Empty<byte>();
    }
}
