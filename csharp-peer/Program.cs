using System;
using System.Net;
using System.Threading;
using Fomoxa.Net;
using Fomoxa.Net.Transports;
using Generated;

namespace CsharpPeer
{
    public static class Program
    {
        private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(10);

        private static Models.Player ServerSends() =>
            new Models.Player { Id = 100, X = 10.5f, Y = 20.0f };

        private static Models.Player ClientSends() =>
            new Models.Player { Id = 200, X = 1.0f, Y = 2.0f };

        public static int Main(string[] args)
        {
            string role = null;
            string addrArg = null;
            string transport = "tcp";
            for (int i = 0; i < args.Length; i++)
            {
                if (args[i] == "--role" && i + 1 < args.Length)
                {
                    role = args[++i];
                }
                else if (args[i] == "--addr" && i + 1 < args.Length)
                {
                    addrArg = args[++i];
                }
                else if (args[i] == "--transport" && i + 1 < args.Length)
                {
                    transport = args[++i];
                }
                else
                {
                    Console.Error.WriteLine($"csharp-peer: unknown argument: {args[i]}");
                    return 2;
                }
            }

            if (role == null)
            {
                Console.Error.WriteLine("csharp-peer: --role server|client is required");
                return 2;
            }
            if (addrArg == null)
            {
                Console.Error.WriteLine("csharp-peer: --addr host:port is required");
                return 2;
            }
            if (transport != "tcp" && transport != "udp")
            {
                Console.Error.WriteLine($"csharp-peer: unknown transport: {transport} (expected tcp or udp)");
                return 2;
            }

            var parts = addrArg.Split(':');
            var endpoint = new IPEndPoint(IPAddress.Parse(parts[0]), int.Parse(parts[1]));
            bool udp = transport == "udp";

            switch (role)
            {
                case "server":
                    return RunServer(endpoint, udp);
                case "client":
                    return RunClient(endpoint, udp);
                default:
                    Console.Error.WriteLine($"csharp-peer: unknown role: {role} (expected server or client)");
                    return 2;
            }
        }

        private static Schema BuildSchema()
        {
            var messages = new MessageSchema[Handshake.FomoxaMessages.Length];
            for (int i = 0; i < messages.Length; i++)
            {
                var message = Handshake.FomoxaMessages[i];
                messages[i] = new MessageSchema(message.Id, message.Fingerprint, message.Prefixes);
            }
            return new Schema(Handshake.FomoxaSchemaFingerprint, messages);
        }

        private static byte[] Encode(Models.Player player)
        {
            var writer = new Writer();
            PlayerEdgeCodec.Encode(writer, player);
            return writer.ToArray();
        }

        private static Models.Player Decode(ReadOnlySpan<byte> bytes)
        {
            var reader = new Reader(bytes);
            var value = new Models.Player();
            PlayerEdgeCodec.Decode(ref reader, ref value);
            return value;
        }

        private static bool SamePlayer(Models.Player a, Models.Player b) =>
            a.Id == b.Id && a.X == b.X && a.Y == b.Y;

        private static int RunServer(IPEndPoint endpoint, bool udp)
        {
            IListenerTransport listener = udp
                ? new UdpServerTransport(endpoint)
                : new TcpListenerTransport(endpoint);
            using var server = new FomoxaServer(listener, BuildSchema(), new SessionConfig());
            Console.WriteLine($"csharp-peer: server listening on {(udp ? "udp" : "tcp")} {endpoint}");

            var expected = ClientSends();
            var deadline = MonotonicClock.Now + Timeout;
            while (MonotonicClock.Now < deadline)
            {
                ulong? readyPeer = null;
                Models.Player received = null;

                foreach (var raised in server.Tick(MonotonicClock.Now))
                {
                    switch (raised.Kind)
                    {
                        case FomoxaEventKind.Ready:
                            Console.WriteLine($"csharp-peer: peer {raised.PeerId} ready, sending Player.edge");
                            readyPeer = raised.PeerId;
                            break;

                        case FomoxaEventKind.Message:
                            received = Decode(raised.Payload.Span);
                            break;

                        case FomoxaEventKind.HandshakeFailed:
                            Console.Error.WriteLine($"csharp-peer: server handshake refused: {raised.Failure}");
                            return 1;
                    }
                }

                if (readyPeer.HasValue)
                {
                    server.Send(readyPeer.Value, PlayerEdgeCodec.MessageId, Encode(ServerSends()));
                }

                if (received != null)
                {
                    if (!SamePlayer(received, expected))
                    {
                        Console.Error.WriteLine(
                            $"csharp-peer: server got unexpected value Id={received.Id} X={received.X} Y={received.Y}");
                        return 1;
                    }
                    Console.WriteLine(
                        $"csharp-peer: server OK, received Id={received.Id} X={received.X} Y={received.Y}");
                    return 0;
                }

                Thread.Sleep(10);
            }

            Console.Error.WriteLine("csharp-peer: server timed out waiting for the client");
            return 1;
        }

        private static int RunClient(IPEndPoint endpoint, bool udp)
        {
            ITransport transport = udp ? UdpTransport.Connect(endpoint) : TcpTransport.Connect(endpoint);
            using var connection = FomoxaConnection.Connect(transport, BuildSchema(), new SessionConfig());
            Console.WriteLine($"csharp-peer: client connected to {(udp ? "udp" : "tcp")} {endpoint}");

            var expected = ServerSends();
            var deadline = MonotonicClock.Now + Timeout;
            while (MonotonicClock.Now < deadline)
            {
                Models.Player received = null;

                foreach (var raised in connection.Tick(MonotonicClock.Now))
                {
                    switch (raised.Kind)
                    {
                        case FomoxaEventKind.Message:
                            received = Decode(raised.Payload.Span);
                            break;

                        case FomoxaEventKind.HandshakeFailed:
                            Console.Error.WriteLine($"csharp-peer: client handshake refused: {raised.Failure}");
                            return 1;
                    }
                }

                if (received != null)
                {
                    if (!SamePlayer(received, expected))
                    {
                        Console.Error.WriteLine(
                            $"csharp-peer: client got unexpected value Id={received.Id} X={received.X} Y={received.Y}");
                        return 1;
                    }
                    Console.WriteLine(
                        $"csharp-peer: client OK, received Id={received.Id} X={received.X} Y={received.Y}, replying");
                    connection.Send(PlayerEdgeCodec.MessageId, Encode(ClientSends()));

                    var drainUntil = MonotonicClock.Now + TimeSpan.FromMilliseconds(200);
                    while (MonotonicClock.Now < drainUntil)
                    {
                        connection.Tick(MonotonicClock.Now);
                        Thread.Sleep(10);
                    }
                    return 0;
                }

                Thread.Sleep(10);
            }

            Console.Error.WriteLine("csharp-peer: client timed out waiting for the server");
            return 1;
        }
    }
}
