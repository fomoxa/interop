using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Fomoxa.Net;
using Fomoxa.Rpc.Demo.Models;
using Generated;

namespace Fomoxa.Rpc.Demo
{
    public static class Proto
    {
        public const uint VoidId = FRpcVoidRpcCodec.MessageId;
        public const uint ErrorId = FRpcErrorRpcCodec.MessageId;
        public const uint CreditId = FRpcCreditRpcCodec.MessageId;
        public const uint ReflectRequest = FRpcReflectRequestRpcCodec.MessageId;
        public const uint ReflectResponse = FRpcReflectResponseRpcCodec.MessageId;

        public const uint EchoRequestId = EchoRequestRpcCodec.MessageId;
        public const uint EchoResponseId = EchoResponseRpcCodec.MessageId;
        public const uint CountRequestId = CountRequestRpcCodec.MessageId;
        public const uint CountItemId = CountItemRpcCodec.MessageId;
        public const uint UploadOpenId = UploadOpenRpcCodec.MessageId;
        public const uint UploadChunkId = UploadChunkRpcCodec.MessageId;
        public const uint UploadDoneId = UploadDoneRpcCodec.MessageId;
        public const uint ChatOpenId = ChatOpenRpcCodec.MessageId;
        public const uint ChatSaidId = ChatSaidRpcCodec.MessageId;
        public const uint ChatHeardId = ChatHeardRpcCodec.MessageId;
        public const uint TicksRequestId = TicksRequestRpcCodec.MessageId;
        public const uint TicksItemId = TicksItemRpcCodec.MessageId;

        public static readonly Method Say = Method.Unary("Echo.Say", EchoRequestId, EchoResponseId);
        public static readonly Method Count = Method.ServerStream("Echo.Count", CountRequestId, CountItemId);
        public static readonly Method Upload =
            Method.ClientStream("Blob.Upload", UploadOpenId, UploadChunkId, UploadDoneId);
        public static readonly Method Chat = Method.BidiStream("Room.Chat", ChatOpenId, ChatSaidId, ChatHeardId);
        public static readonly Method Ticks = Method.ServerStream("Clock.Ticks", TicksRequestId, TicksItemId);
        public static readonly Method Reflect = Method.Unary(Reflection.MethodName, ReflectRequest, ReflectResponse);

        public static IReadOnlyList<Method> Methods { get; } = new[] { Say, Count, Upload, Chat, Ticks };

        public static byte[] SchemaJson { get; } = LoadSchemaJson();

        public static Schema NetSchema() =>
            new Schema(
                Handshake.FomoxaSchemaFingerprint,
                Handshake.FomoxaMessages.Select(message =>
                    new MessageSchema(message.Id, message.Fingerprint, message.Prefixes)));

        public static Registry CallerRegistry()
        {
            var builder = Registry.Builder(VoidId, ErrorId).Credit(CreditId).Declare(Reflect);
            foreach (var method in Methods)
            {
                builder.Declare(method);
            }
            return builder.Build();
        }

        public static Registry ResponderRegistry(bool reflection = true)
        {
            var builder = Registry.Builder(VoidId, ErrorId)
                .Credit(CreditId)
                .Serve(Say, (_, body) =>
                {
                    string text = DecodeEcho(body);
                    if (text.Length == 0)
                    {
                        throw new StatusException(Status.InvalidArgument("Echo.Say needs a non-empty text"));
                    }
                    return Immediate.Respond(EncodeEchoResponse(text.ToUpperInvariant()));
                })
                .Serve(Count, (_, body) =>
                {
                    uint count = DecodeCount(body);
                    if (count > 100)
                    {
                        throw new StatusException(Status.InvalidArgument("Echo.Count is capped at 100"));
                    }
                    return new Counter(count);
                })
                .Serve(Upload, (_, _) => new UploadHandler())
                .Serve(Chat, (_, _) => new ChatHandler())
                .Serve(Ticks, (_, _) => new TicksHandler());
            if (reflection)
            {
                builder.Reflection(ReflectRequest, ReflectResponse, SchemaJson);
            }
            return builder.Build();
        }

        public static byte[] EncodeEcho(string text) => Encode(writer => EchoRequestRpcCodec.Encode(writer, new EchoRequest { Text = text }));

        public static string DecodeEcho(byte[] body)
        {
            var value = new EchoRequest();
            var reader = new Reader(body);
            EchoRequestRpcCodec.Decode(ref reader, ref value);
            return value.Text;
        }

        public static byte[] EncodeEchoResponse(string text) =>
            Encode(writer => EchoResponseRpcCodec.Encode(writer, new EchoResponse { Text = text }));

        public static string DecodeEchoResponse(byte[] body)
        {
            var value = new EchoResponse();
            var reader = new Reader(body);
            EchoResponseRpcCodec.Decode(ref reader, ref value);
            return value.Text;
        }

        public static byte[] EncodeCount(uint count) =>
            Encode(writer => CountRequestRpcCodec.Encode(writer, new CountRequest { Count = count }));

        public static uint DecodeCount(byte[] body)
        {
            var value = new CountRequest();
            var reader = new Reader(body);
            CountRequestRpcCodec.Decode(ref reader, ref value);
            return value.Count;
        }

        public static byte[] EncodeCountItem(uint value) =>
            Encode(writer => CountItemRpcCodec.Encode(writer, new CountItem { Value = value }));

        public static uint DecodeCountItem(byte[] body)
        {
            var value = new CountItem();
            var reader = new Reader(body);
            CountItemRpcCodec.Decode(ref reader, ref value);
            return value.Value;
        }

        public static byte[] EncodeUploadOpen(string name) =>
            Encode(writer => UploadOpenRpcCodec.Encode(writer, new UploadOpen { Name = name }));

        public static byte[] EncodeChunk(uint size) =>
            Encode(writer => UploadChunkRpcCodec.Encode(writer, new UploadChunk { Size = size }));

        public static uint DecodeChunk(byte[] body)
        {
            var value = new UploadChunk();
            var reader = new Reader(body);
            UploadChunkRpcCodec.Decode(ref reader, ref value);
            return value.Size;
        }

        public static byte[] EncodeDone(string summary) =>
            Encode(writer => UploadDoneRpcCodec.Encode(writer, new UploadDone { Summary = summary }));

        public static string DecodeDone(byte[] body)
        {
            var value = new UploadDone();
            var reader = new Reader(body);
            UploadDoneRpcCodec.Decode(ref reader, ref value);
            return value.Summary;
        }

        public static byte[] EncodeRoom(string room) =>
            Encode(writer => ChatOpenRpcCodec.Encode(writer, new ChatOpen { Room = room }));

        public static byte[] EncodeSaid(string line) =>
            Encode(writer => ChatSaidRpcCodec.Encode(writer, new ChatSaid { Line = line }));

        public static string DecodeSaid(byte[] body)
        {
            var value = new ChatSaid();
            var reader = new Reader(body);
            ChatSaidRpcCodec.Decode(ref reader, ref value);
            return value.Line;
        }

        public static byte[] EncodeHeard(string line) =>
            Encode(writer => ChatHeardRpcCodec.Encode(writer, new ChatHeard { Line = line }));

        public static string DecodeHeard(byte[] body)
        {
            var value = new ChatHeard();
            var reader = new Reader(body);
            ChatHeardRpcCodec.Decode(ref reader, ref value);
            return value.Line;
        }

        public static byte[] EncodeTicksRequest() =>
            Encode(writer => TicksRequestRpcCodec.Encode(writer, new TicksRequest()));

        public static byte[] EncodeTick(uint value) =>
            Encode(writer => TicksItemRpcCodec.Encode(writer, new TicksItem { Value = value }));

        public static uint DecodeTick(byte[] body)
        {
            var value = new TicksItem();
            var reader = new Reader(body);
            TicksItemRpcCodec.Decode(ref reader, ref value);
            return value.Value;
        }

        public static byte[] EncodeReflectRequest() =>
            Encode(writer => FRpcReflectRequestRpcCodec.Encode(writer, new FRpcReflectRequest()));

        public static FRpcReflectResponse DecodeReflectResponse(byte[] body)
        {
            var value = new FRpcReflectResponse();
            var reader = new Reader(body);
            FRpcReflectResponseRpcCodec.Decode(ref reader, ref value);
            return value;
        }

        private static byte[] Encode(Action<Writer> write)
        {
            var writer = new Writer();
            write(writer);
            return writer.ToArray();
        }

        private static byte[] LoadSchemaJson()
        {
            using var stream = typeof(Proto).Assembly.GetManifestResourceStream("schema.json")
                ?? throw new InvalidOperationException("schema.json is not embedded");
            using var copy = new MemoryStream();
            stream.CopyTo(copy);
            return copy.ToArray();
        }

        private sealed class Counter : IHandler
        {
            private readonly uint count;
            private uint next;

            public Counter(uint count)
            {
                this.count = count;
            }

            public Progress Poll(TimeSpan now)
            {
                if (next >= count)
                {
                    return Progress.End;
                }
                next++;
                return Progress.Item(EncodeCountItem(next));
            }
        }

        private sealed class UploadHandler : IHandler
        {
            private uint bytes;
            private uint chunks;
            private bool closed;

            public Progress Poll(TimeSpan now) =>
                closed ? Progress.Respond(EncodeDone($"{chunks} chunks, {bytes} bytes")) : Progress.Pending;

            public void Item(byte[] body)
            {
                uint size = DecodeChunk(body);
                if (size == 0)
                {
                    throw new StatusException(Status.InvalidArgument("an empty chunk is not accepted"));
                }
                chunks++;
                bytes += size;
            }

            public void EndOfStream() => closed = true;
        }

        private sealed class ChatHandler : IHandler
        {
            private readonly Queue<byte[]> replies = new Queue<byte[]>();
            private bool closed;

            public Progress Poll(TimeSpan now)
            {
                if (replies.Count > 0)
                {
                    return Progress.Item(replies.Dequeue());
                }
                return closed ? Progress.End : Progress.Pending;
            }

            public void Item(byte[] body) => replies.Enqueue(EncodeHeard(DecodeSaid(body).ToUpperInvariant()));

            public void EndOfStream() => closed = true;
        }

        private sealed class TicksHandler : IHandler
        {
            private uint next;

            public Progress Poll(TimeSpan now)
            {
                next++;
                return Progress.Item(EncodeTick(next));
            }
        }
    }
}
