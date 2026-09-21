using Fomoxa;

namespace Fomoxa.Rpc.Demo.Models
{
    [Network]
    [Codec("rpc")]
    public class EchoRequest
    {
        [Network("string")]
        [Codec("rpc")]
        public string Text { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class EchoResponse
    {
        [Network("string")]
        [Codec("rpc")]
        public string Text { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class CountRequest
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint Count { get; set; }
    }

    [Network]
    [Codec("rpc")]
    public class CountItem
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint Value { get; set; }
    }
}
