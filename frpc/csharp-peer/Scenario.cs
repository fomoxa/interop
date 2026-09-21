using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Fomoxa.Rpc.Net;
using Generated;

namespace Fomoxa.Rpc.Demo
{
    public static class Scenario
    {
        public static readonly Method Missing = Method.Unary("Echo.Missing", FRpcCtxRpcCodec.MessageId, Proto.EchoResponseId);

        public static Registry DriverRegistry() =>
            Registry.Builder(Proto.VoidId, Proto.ErrorId)
                .Credit(Proto.CreditId)
                .Declare(Proto.Reflect)
                .Declare(Proto.Say)
                .Declare(Proto.Count)
                .Declare(Proto.Upload)
                .Declare(Proto.Chat)
                .Declare(Proto.Ticks)
                .Declare(Missing)
                .Build();

        public static int Drive(RpcClient client, TextWriter log)
        {
            var checks = new List<(string Name, Func<RpcClient, string?> Body)>
            {
                ("unary Echo.Say", SayUppercases),
                ("unary error Echo.Say empty", SayRejectsEmpty),
                ("server stream Echo.Count", CountStreams),
                ("server stream error Echo.Count 101", CountRejectsLarge),
                ("client stream Blob.Upload", UploadSummarises),
                ("client stream error Blob.Upload empty chunk", UploadRejectsEmptyChunk),
                ("bidi Room.Chat", ChatEchoes),
                ("credit Clock.Ticks", TicksFollowCredit),
                ("deadline Clock.Ticks", TicksExpire),
                ("unimplemented Echo.Missing", MissingIsUnimplemented),
                ("reflection FRpc.Reflect", ReflectionLists),
            };
            int failures = 0;
            foreach (var (name, body) in checks)
            {
                string? problem;
                try
                {
                    problem = body(client);
                }
                catch (Exception crashed)
                {
                    problem = crashed.Message;
                }
                if (problem == null)
                {
                    log.WriteLine($"ok   {name}");
                }
                else
                {
                    failures++;
                    log.WriteLine($"FAIL {name}: {problem}");
                }
            }
            return failures;
        }

        public static IReadOnlyList<string> Frames()
        {
            var now = TimeSpan.Zero;
            var client = new RpcCore(DriverRegistry(), Role.Client);
            var server = new RpcCore(Proto.ResponderRegistry(false), Role.Server);
            client.OnReady();
            server.OnReady();
            var lines = new List<string>();

            var traced = new Ctx
            {
                DeadlineMs = 1500,
                TraceId = new byte[] { 1, 2, 3, 4 },
                SpanId = new byte[] { 5, 6 },
                Token = "tok",
                Tenant = "acme",
            };
            client.Call(Proto.EchoRequestId, Proto.EncodeEcho("xin chao"), traced, CallOptions.WithoutDeadline, now);
            var ticks = client.Call(
                Proto.TicksRequestId, Proto.EncodeTicksRequest(), Ctx.WithInitialCredit(2), CallOptions.WithoutDeadline, now).Call;
            client.Grant(ticks, 3);
            var upload = client.Call(Proto.UploadOpenId, Proto.EncodeUploadOpen("photo.raw"), null, CallOptions.WithoutDeadline, now).Call;
            client.SendItem(upload, Proto.EncodeChunk(4096));
            client.SendItem(upload, Proto.EncodeChunk(1024));
            client.CloseSend(upload);
            client.Call(Proto.EchoRequestId, Proto.EncodeEcho(""), null, CallOptions.WithoutDeadline, now);
            client.Call(Missing.RequestId, Array.Empty<byte>(), null, CallOptions.WithoutDeadline, now);
            var chat = client.Call(Proto.ChatOpenId, Proto.EncodeRoom("lobby"), null, CallOptions.WithoutDeadline, now).Call;
            client.SendItem(chat, Proto.EncodeSaid("hello"));
            client.CloseSend(chat);

            Exchange(client, server, now, lines);
            client.Cancel(ticks);
            Exchange(client, server, now, lines);
            return lines;
        }

        private static void Exchange(RpcCore client, RpcCore server, TimeSpan now, List<string> lines)
        {
            foreach (var frame in Take(client, "C", lines))
            {
                server.OnMessage(frame.MessageId, frame.Payload, now);
            }
            server.Tick(now);
            foreach (var frame in Take(server, "S", lines))
            {
                client.OnMessage(frame.MessageId, frame.Payload, now);
            }
            client.Tick(now);
            client.TakeOutcomes();
        }

        private static List<Outgoing> Take(RpcCore core, string side, List<string> lines)
        {
            var frames = new List<Outgoing>();
            core.Drain(frame =>
            {
                frames.Add(frame);
                lines.Add($"{side} {frame.MessageId:X8} {Convert.ToHexString(frame.Payload)}");
                return Delivery.Accepted;
            });
            return frames;
        }

        private static string? SayUppercases(RpcClient client)
        {
            var outcome = client.Invoke(Proto.EchoRequestId, Proto.EncodeEcho("xin chao"));
            if (outcome.Kind != OutcomeKind.Response)
            {
                return outcome.ToString();
            }
            string text = Proto.DecodeEchoResponse(outcome.Body);
            return text == "XIN CHAO" ? null : $"answered {text}";
        }

        private static string? SayRejectsEmpty(RpcClient client) =>
            ExpectStatus(client.Invoke(Proto.EchoRequestId, Proto.EncodeEcho("")), StatusCode.InvalidArgument);

        private static string? CountStreams(RpcClient client)
        {
            var call = Open(client, Proto.CountRequestId, Proto.EncodeCount(5), null);
            var seen = new List<uint>();
            foreach (var outcome in client.Collect(call))
            {
                switch (outcome.Kind)
                {
                    case OutcomeKind.Item:
                        seen.Add(Proto.DecodeCountItem(outcome.Body));
                        break;
                    case OutcomeKind.End:
                        return seen.SequenceEqual(new uint[] { 1, 2, 3, 4, 5 }) ? null : $"items {string.Join(",", seen)}";
                    default:
                        return outcome.ToString();
                }
            }
            return "the stream ended without END";
        }

        private static string? CountRejectsLarge(RpcClient client) =>
            ExpectStatus(client.Next(Open(client, Proto.CountRequestId, Proto.EncodeCount(101), null)), StatusCode.InvalidArgument);

        private static string? UploadSummarises(RpcClient client)
        {
            var call = Open(client, Proto.UploadOpenId, Proto.EncodeUploadOpen("photo.raw"), null);
            foreach (uint size in new uint[] { 4096, 8192, 1024 })
            {
                var sent = client.SendItem(call, Proto.EncodeChunk(size));
                if (sent != CallError.None)
                {
                    return $"sending a chunk: {sent}";
                }
            }
            client.CloseSend(call);
            var outcome = client.Next(call);
            if (outcome.Kind != OutcomeKind.Response)
            {
                return outcome.ToString();
            }
            string summary = Proto.DecodeDone(outcome.Body);
            return summary == "3 chunks, 13312 bytes" ? null : $"answered {summary}";
        }

        private static string? UploadRejectsEmptyChunk(RpcClient client)
        {
            var call = Open(client, Proto.UploadOpenId, Proto.EncodeUploadOpen("empty.raw"), null);
            client.SendItem(call, Proto.EncodeChunk(0));
            return ExpectStatus(client.Next(call), StatusCode.InvalidArgument);
        }

        private static string? ChatEchoes(RpcClient client)
        {
            var call = Open(client, Proto.ChatOpenId, Proto.EncodeRoom("lobby"), null);
            foreach (string line in new[] { "hello", "still here", "bye" })
            {
                client.SendItem(call, Proto.EncodeSaid(line));
                var heard = client.Next(call);
                if (heard.Kind != OutcomeKind.Item)
                {
                    return heard.ToString();
                }
                string text = Proto.DecodeHeard(heard.Body);
                if (text != line.ToUpperInvariant())
                {
                    return $"heard {text} for {line}";
                }
            }
            client.CloseSend(call);
            var last = client.Next(call);
            return last.Kind == OutcomeKind.End ? null : last.ToString();
        }

        private static string? TicksFollowCredit(RpcClient client)
        {
            var call = Open(client, Proto.TicksRequestId, Proto.EncodeTicksRequest(), Ctx.WithInitialCredit(2));
            var seen = new List<uint> { NextTick(client, call), NextTick(client, call) };
            var quietUntil = MonotonicNow() + TimeSpan.FromMilliseconds(150);
            while (MonotonicNow() < quietUntil)
            {
                client.Session.Wait(WaitPolicy.Default, MonotonicNow());
                client.Pump();
            }
            if (client.Core.PendingRequestId(call) == null)
            {
                return "the stream ended while the allowance was zero";
            }
            client.Grant(call, 3);
            seen.Add(NextTick(client, call));
            seen.Add(NextTick(client, call));
            seen.Add(NextTick(client, call));
            client.Cancel(call);
            var cancelled = client.Next(call);
            if (cancelled.Status?.Code != StatusCode.Cancelled)
            {
                return $"after cancel: {cancelled}";
            }
            return seen.SequenceEqual(new uint[] { 1, 2, 3, 4, 5 }) ? null : $"items {string.Join(",", seen)}";
        }

        private static string? TicksExpire(RpcClient client)
        {
            var opened = client.Call(
                Proto.TicksRequestId, Proto.EncodeTicksRequest(), Ctx.WithInitialCredit(1), CallOptions.Within(TimeSpan.FromMilliseconds(200)));
            if (!opened.IsOk)
            {
                return opened.ToString();
            }
            NextTick(client, opened.Call);
            return ExpectStatus(client.Next(opened.Call), StatusCode.DeadlineExceeded);
        }

        private static string? MissingIsUnimplemented(RpcClient client) =>
            ExpectStatus(client.Invoke(Missing.RequestId, Array.Empty<byte>()), StatusCode.Unimplemented);

        private static string? ReflectionLists(RpcClient client)
        {
            var outcome = client.Invoke(Proto.ReflectRequest, Proto.EncodeReflectRequest());
            if (outcome.Kind != OutcomeKind.Response)
            {
                return outcome.ToString();
            }
            var answer = Proto.DecodeReflectResponse(outcome.Body);
            var expected = Proto.Methods.Append(Proto.Reflect.Idempotent())
                .Select(method => $"{method.Name}/{(uint)method.Shape}/{method.RequestId:X8}/{method.CallerItemId:X8}/{method.ReplyId:X8}/{(uint)method.RetrySafety}")
                .OrderBy(text => text, StringComparer.Ordinal);
            var listed = answer.Methods
                .Select(info => $"{info.Name}/{info.Shape}/{info.RequestId:X8}/{info.CallerItemId:X8}/{info.ReplyId:X8}/{info.RetrySafety}")
                .OrderBy(text => text, StringComparer.Ordinal);
            if (!expected.SequenceEqual(listed))
            {
                return $"listed {string.Join(" ", listed)}";
            }
            using var schema = JsonDocument.Parse(answer.SchemaJson);
            string fingerprint = schema.RootElement.GetProperty("fingerprint_u64").GetString() ?? "";
            return fingerprint == $"0x{Handshake.FomoxaSchemaFingerprint:X16}" ? null : $"schema fingerprint {fingerprint}";
        }

        private static CallId Open(RpcClient client, uint requestId, byte[] body, Ctx? ctx)
        {
            var opened = client.Call(requestId, body, ctx, CallOptions.Default);
            if (!opened.IsOk)
            {
                throw new InvalidOperationException($"the call could not be sent: {opened}");
            }
            return opened.Call;
        }

        private static uint NextTick(RpcClient client, CallId call)
        {
            var outcome = client.Next(call);
            if (outcome.Kind != OutcomeKind.Item)
            {
                throw new InvalidOperationException($"expected a tick, got {outcome}");
            }
            return Proto.DecodeTick(outcome.Body);
        }

        private static string? ExpectStatus(Outcome outcome, uint code) =>
            outcome.Kind == OutcomeKind.Failed && outcome.Status?.Code == code
                ? null
                : $"expected {StatusCode.NameOf(code)}, got {outcome}";

        private static TimeSpan MonotonicNow() => Fomoxa.Net.MonotonicClock.Now;
    }
}
