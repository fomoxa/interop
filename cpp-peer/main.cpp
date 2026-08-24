#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#include "fomoxa/net.hpp"

#include "src/generated/handshake.hpp"
#include "src/generated/player_edge.hpp"
#include "src/models/player.hpp"

namespace {

constexpr std::uint64_t kTimeoutMs = 10000;

const models::Player SERVER_SENDS{100, 10.5f, 20.0f};
const models::Player CLIENT_SENDS{200, 1.0f, 2.0f};

bool same_player(const models::Player &a, const models::Player &b) {
    return a.Id == b.Id && a.X == b.X && a.Y == b.Y;
}

std::vector<fmx_message_schema> g_messages;
fmx_schema g_schema;

const fmx_schema *peer_schema() {
    g_messages.clear();
    for (const auto &message : generated::FOMOXA_MESSAGES) {
        fmx_message_schema m;
        m.id = message.id;
        m.fingerprint = message.fingerprint;
        m.prefixes = message.prefixes;
        m.prefix_count = message.prefix_count;
        g_messages.push_back(m);
    }
    g_schema.fingerprint = generated::FOMOXA_SCHEMA_FINGERPRINT;
    g_schema.messages = g_messages.data();
    g_schema.message_count = g_messages.size();
    return &g_schema;
}

std::vector<std::uint8_t> encode_player(const models::Player &player) {
    generated::Writer writer;
    generated::PlayerEdgeCodec::encode(writer, player);
    return writer.bytes();
}

models::Player decode_player(fomoxa::ByteView payload) {
    generated::Reader reader(payload.data(), payload.size());
    models::Player value;
    generated::DecodeError error = generated::PlayerEdgeCodec::decode(reader, value);
    if (!error.ok()) {
        std::fprintf(stderr, "cpp-peer: undecodable Player.edge\n");
        std::exit(1);
    }
    return value;
}

void split_addr(const std::string &addr, std::string &host, std::uint16_t &port) {
    auto pos = addr.rfind(':');
    if (pos == std::string::npos) {
        std::fprintf(stderr, "cpp-peer: invalid --addr %s (expected host:port)\n", addr.c_str());
        std::exit(2);
    }
    host = addr.substr(0, pos);
    port = static_cast<std::uint16_t>(std::stoi(addr.substr(pos + 1)));
}

void nap() {
    std::this_thread::sleep_for(std::chrono::milliseconds(16));
}

void run_server(const std::string &host, std::uint16_t port, bool udp) {
    auto listener = udp ? fomoxa::Listener::udp(host, port) : fomoxa::Listener::tcp(host, port);
    if (!listener) {
        std::fprintf(stderr, "cpp-peer: bind the interop address\n");
        std::exit(1);
    }
    auto server = fomoxa::Server::create(std::move(*listener), peer_schema());
    if (!server) {
        std::fprintf(stderr, "cpp-peer: cannot create the server\n");
        std::exit(1);
    }
    std::printf("cpp-peer: server listening on %s:%u\n", host.c_str(), (unsigned)port);

    std::uint64_t deadline = fomoxa::now_ms() + kTimeoutMs;
    while (fomoxa::now_ms() < deadline) {
        bool have_ready = false;
        std::uint64_t ready_peer = 0;
        bool have_received = false;
        models::Player received;

        for (const auto &event : server->tick(fomoxa::now_ms())) {
            switch (event.kind()) {
            case FMX_EVENT_READY:
                std::printf("cpp-peer: peer#%llu ready, sending Player.edge\n",
                            (unsigned long long)event.peer());
                have_ready = true;
                ready_peer = event.peer();
                break;
            case FMX_EVENT_MESSAGE:
                if (event.message_id() == generated::PlayerEdgeCodec::kMessageId) {
                    received = decode_player(event.payload());
                    have_received = true;
                }
                break;
            case FMX_EVENT_HANDSHAKE_FAILED:
                std::fprintf(stderr, "cpp-peer: server handshake refused: %s\n",
                             fmx_handshake_failure_name(event.handshake_failure()));
                std::exit(1);
            default:
                break;
            }
        }

        if (have_ready) {
            auto payload = encode_player(SERVER_SENDS);
            server->send(ready_peer, generated::PlayerEdgeCodec::kMessageId,
                         fomoxa::ByteView(payload.data(), payload.size()));
        }

        if (have_received) {
            if (!same_player(received, CLIENT_SENDS)) {
                std::fprintf(stderr, "cpp-peer: server got unexpected value id=%u x=%g y=%g\n",
                             received.Id, (double)received.X, (double)received.Y);
                std::exit(1);
            }
            std::printf("cpp-peer: server OK, received id=%u x=%g y=%g\n", received.Id,
                        (double)received.X, (double)received.Y);
            return;
        }

        nap();
    }

    std::fprintf(stderr, "cpp-peer: server timed out waiting for the client\n");
    std::exit(1);
}

void run_client(const std::string &host, std::uint16_t port, bool udp) {
    auto transport = udp ? fomoxa::Transport::udp(host, port) : fomoxa::Transport::tcp(host, port);
    if (!transport) {
        std::fprintf(stderr, "cpp-peer: connect to the interop address\n");
        std::exit(1);
    }
    auto connection = fomoxa::Connection::create(std::move(*transport), peer_schema());
    if (!connection) {
        std::fprintf(stderr, "cpp-peer: cannot create the connection\n");
        std::exit(1);
    }
    std::printf("cpp-peer: client connected to %s:%u\n", host.c_str(), (unsigned)port);

    std::uint64_t deadline = fomoxa::now_ms() + kTimeoutMs;
    while (fomoxa::now_ms() < deadline) {
        bool have_received = false;
        models::Player received;

        for (const auto &event : connection->tick(fomoxa::now_ms())) {
            switch (event.kind()) {
            case FMX_EVENT_MESSAGE:
                if (event.message_id() == generated::PlayerEdgeCodec::kMessageId) {
                    received = decode_player(event.payload());
                    have_received = true;
                }
                break;
            case FMX_EVENT_HANDSHAKE_FAILED:
                std::fprintf(stderr, "cpp-peer: client handshake refused: %s\n",
                             fmx_handshake_failure_name(event.handshake_failure()));
                std::exit(1);
            default:
                break;
            }
        }

        if (have_received) {
            if (!same_player(received, SERVER_SENDS)) {
                std::fprintf(stderr, "cpp-peer: client got unexpected value id=%u x=%g y=%g\n",
                             received.Id, (double)received.X, (double)received.Y);
                std::exit(1);
            }
            std::printf("cpp-peer: client OK, received id=%u x=%g y=%g, replying\n", received.Id,
                        (double)received.X, (double)received.Y);

            auto payload = encode_player(CLIENT_SENDS);
            connection->send(generated::PlayerEdgeCodec::kMessageId,
                             fomoxa::ByteView(payload.data(), payload.size()));

            std::uint64_t drain_until = fomoxa::now_ms() + 200;
            while (fomoxa::now_ms() < drain_until) {
                connection->tick(fomoxa::now_ms());
                nap();
            }
            return;
        }

        nap();
    }

    std::fprintf(stderr, "cpp-peer: client timed out waiting for the server\n");
    std::exit(1);
}

}  // namespace

int main(int argc, char **argv) {
    std::string role;
    std::string addr;
    std::string transport = "tcp";

    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--role" && i + 1 < argc) {
            role = argv[++i];
        } else if (arg == "--addr" && i + 1 < argc) {
            addr = argv[++i];
        } else if (arg == "--transport" && i + 1 < argc) {
            transport = argv[++i];
        } else {
            std::fprintf(stderr, "cpp-peer: unknown argument: %s\n", arg.c_str());
            return 2;
        }
    }
    if (role.empty()) {
        std::fprintf(stderr, "cpp-peer: --role server|client is required\n");
        return 2;
    }
    if (addr.empty()) {
        std::fprintf(stderr, "cpp-peer: --addr host:port is required\n");
        return 2;
    }
    if (transport != "tcp" && transport != "udp") {
        std::fprintf(stderr, "cpp-peer: unknown transport: %s (expected tcp or udp)\n",
                     transport.c_str());
        return 2;
    }

    std::string host;
    std::uint16_t port = 0;
    split_addr(addr, host, port);
    bool udp = transport == "udp";

    if (role == "server") {
        run_server(host, port, udp);
    } else if (role == "client") {
        run_client(host, port, udp);
    } else {
        std::fprintf(stderr, "cpp-peer: unknown role: %s (expected server or client)\n",
                     role.c_str());
        return 2;
    }

    std::printf("cpp-peer: done\n");
    return 0;
}
