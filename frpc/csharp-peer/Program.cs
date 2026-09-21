using System;
using System.Threading;
using Fomoxa.Rpc.Demo;
using Fomoxa.Rpc.Net;

namespace Fomoxa.Rpc.Interop
{
    public static class Program
    {
        public static int Main(string[] args)
        {
            if (args.Length == 2 && args[0] == "serve")
            {
                return Serve(args[1]);
            }
            if (args.Length == 2 && args[0] == "drive")
            {
                return Drive(args[1]);
            }
            if (args.Length == 1 && args[0] == "frames")
            {
                foreach (string line in Scenario.Frames())
                {
                    Console.WriteLine(line);
                }
                return 0;
            }
            if (args.Length == 3 && args[0] == "say")
            {
                return Say(args[1], args[2]);
            }
            Console.Error.WriteLine("usage: frpc-interop serve <host:port> | drive <host:port> | frames | say <host:port> <text>");
            return 64;
        }

        private static int Serve(string address)
        {
            var (host, port) = Split(address);
            using var server = ThreadedServer.Bind(host, port, Proto.NetSchema(), Proto.ResponderRegistry());
            var runner = server.Start();
            Console.WriteLine($"listening {server.LocalEndPoint}");
            Console.Out.Flush();
            Console.In.ReadToEnd();
            server.Stop();
            runner.Join(TimeSpan.FromSeconds(2));
            return 0;
        }

        private static int Drive(string address)
        {
            var (host, port) = Split(address);
            using var client = RpcClient.Connect(
                host, port, Proto.NetSchema(), Scenario.DriverRegistry(), TimeSpan.FromSeconds(5));
            int failures = Scenario.Drive(client, Console.Out);
            Console.WriteLine(failures == 0 ? "all checks passed" : $"{failures} checks failed");
            return failures == 0 ? 0 : 1;
        }

        private static int Say(string address, string text)
        {
            var (host, port) = Split(address);
            using var client = RpcClient.Connect(
                host, port, Proto.NetSchema(), Scenario.DriverRegistry(), TimeSpan.FromSeconds(5));
            var outcome = client.Invoke(Proto.EchoRequestId, Proto.EncodeEcho(text));
            if (outcome.Kind != OutcomeKind.Response)
            {
                Console.Error.WriteLine(outcome);
                return 1;
            }
            Console.WriteLine(Proto.DecodeEchoResponse(outcome.Body));
            return 0;
        }

        private static (string Host, int Port) Split(string address)
        {
            int colon = address.LastIndexOf(':');
            return (address.Substring(0, colon), int.Parse(address.Substring(colon + 1)));
        }
    }
}
