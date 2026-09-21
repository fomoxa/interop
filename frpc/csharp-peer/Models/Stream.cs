using Fomoxa;

namespace Fomoxa.Rpc.Demo.Models
{
    [Network]
    [Codec("rpc")]
    public class UploadOpen
    {
        [Network("string")]
        [Codec("rpc")]
        public string Name { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class UploadChunk
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint Size { get; set; }
    }

    [Network]
    [Codec("rpc")]
    public class UploadDone
    {
        [Network("string")]
        [Codec("rpc")]
        public string Summary { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class ChatOpen
    {
        [Network("string")]
        [Codec("rpc")]
        public string Room { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class ChatSaid
    {
        [Network("string")]
        [Codec("rpc")]
        public string Line { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class ChatHeard
    {
        [Network("string")]
        [Codec("rpc")]
        public string Line { get; set; } = "";
    }

    [Network]
    [Codec("rpc")]
    public class TicksRequest
    {
    }

    [Network]
    [Codec("rpc")]
    public class TicksItem
    {
        [Network("u32")]
        [Codec("rpc")]
        public uint Value { get; set; }
    }
}
