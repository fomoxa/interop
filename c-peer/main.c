#if !defined(_WIN32)
#define _POSIX_C_SOURCE 200809L
#endif

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fomoxa/net.h"

#include "src/generated/handshake.h"
#include "src/generated/player_edge.h"
#include "src/models/player.h"

#if defined(_WIN32)
#include <windows.h>
static void nap(void) {
    Sleep(16);
}
#else
#include <time.h>
static void nap(void) {
    struct timespec pause;
    pause.tv_sec = 0;
    pause.tv_nsec = 16000000L;
    nanosleep(&pause, NULL);
}
#endif

#define TIMEOUT_MS 10000

static const struct Player SERVER_SENDS = {100, 10.5f, 20.0f};
static const struct Player CLIENT_SENDS = {200, 1.0f, 2.0f};

static fmx_message_schema PEER_MESSAGES[sizeof(FOMOXA_MESSAGES) / sizeof(FOMOXA_MESSAGES[0])];
static fmx_schema PEER_SCHEMA;

static const fmx_schema *peer_schema(void) {
    size_t index;
    for (index = 0; index < FOMOXA_MESSAGES_COUNT; ++index) {
        PEER_MESSAGES[index].id = FOMOXA_MESSAGES[index].id;
        PEER_MESSAGES[index].fingerprint = FOMOXA_MESSAGES[index].fingerprint;
        PEER_MESSAGES[index].prefixes = FOMOXA_MESSAGES[index].prefixes;
        PEER_MESSAGES[index].prefix_count = FOMOXA_MESSAGES[index].prefix_count;
    }
    PEER_SCHEMA.fingerprint = FOMOXA_SCHEMA_FINGERPRINT;
    PEER_SCHEMA.messages = PEER_MESSAGES;
    PEER_SCHEMA.message_count = FOMOXA_MESSAGES_COUNT;
    return &PEER_SCHEMA;
}

static int same_player(const struct Player *a, const struct Player *b) {
    return a->Id == b->Id && a->X == b->X && a->Y == b->Y;
}

static void encode_player(const struct Player *player, FomoxaWriter *writer) {
    fomoxa_writer_init(writer);
    PlayerEdgeCodec_encode(writer, player);
}

static void decode_player(const uint8_t *payload, size_t len, struct Player *out) {
    FomoxaReader reader;
    FomoxaDecodeError error;
    memset(out, 0, sizeof(*out));
    fomoxa_reader_init(&reader, payload, len, fomoxa_limits_unlimited());
    error = PlayerEdgeCodec_decode(&reader, out);
    if (!fomoxa_decode_error_ok(&error)) {
        fprintf(stderr, "c-peer: undecodable Player.edge\n");
        exit(1);
    }
}

static void split_addr(const char *addr, char *host, size_t host_size, uint16_t *port) {
    const char *colon = strrchr(addr, ':');
    size_t host_len;
    if (colon == NULL) {
        fprintf(stderr, "c-peer: invalid --addr %s (expected host:port)\n", addr);
        exit(2);
    }
    host_len = (size_t)(colon - addr);
    if (host_len >= host_size) {
        host_len = host_size - 1;
    }
    memcpy(host, addr, host_len);
    host[host_len] = '\0';
    *port = (uint16_t)atoi(colon + 1);
}

static void run_server(const char *host, uint16_t port, int udp) {
    fmx_listener listener;
    fmx_server *server;
    fmx_result result;
    uint64_t deadline;

    result = udp ? fmx_udp_listen(host, port, &listener) : fmx_tcp_listen(host, port, &listener);
    if (result != FMX_OK) {
        fprintf(stderr, "c-peer: bind the interop address: %s\n", fmx_result_name(result));
        exit(1);
    }
    server = fmx_server_create(listener, peer_schema(), NULL);
    if (server == NULL) {
        fprintf(stderr, "c-peer: cannot create the server\n");
        exit(1);
    }
    printf("c-peer: server listening on %s:%u\n", host, (unsigned)port);

    deadline = fmx_now_ms() + TIMEOUT_MS;
    while (fmx_now_ms() < deadline) {
        const fmx_event *events;
        size_t count, index;
        int have_ready = 0;
        uint64_t ready_peer = 0;
        int have_received = 0;
        struct Player received;

        fmx_server_tick(server, fmx_now_ms());
        events = fmx_server_events(server, &count);
        for (index = 0; index < count; ++index) {
            const fmx_event *event = &events[index];
            switch (event->kind) {
            case FMX_EVENT_READY:
                printf("c-peer: peer#%llu ready, sending Player.edge\n",
                       (unsigned long long)event->peer);
                have_ready = 1;
                ready_peer = event->peer;
                break;
            case FMX_EVENT_MESSAGE:
                if (event->message_id == PlayerEdgeCodec_MESSAGE_ID) {
                    decode_player(event->payload, event->payload_len, &received);
                    have_received = 1;
                }
                break;
            case FMX_EVENT_HANDSHAKE_FAILED:
                fprintf(stderr, "c-peer: server handshake refused: %s\n",
                        fmx_handshake_failure_name((fmx_handshake_failure)event->reason));
                exit(1);
            default:
                break;
            }
        }

        if (have_ready) {
            FomoxaWriter writer;
            encode_player(&SERVER_SENDS, &writer);
            (void)fmx_server_send(server, ready_peer, PlayerEdgeCodec_MESSAGE_ID, writer.data,
                                  writer.len);
            fomoxa_writer_free(&writer);
        }

        if (have_received) {
            if (!same_player(&received, &CLIENT_SENDS)) {
                fprintf(stderr, "c-peer: server got unexpected value id=%u x=%g y=%g\n",
                        (unsigned)received.Id, (double)received.X, (double)received.Y);
                exit(1);
            }
            printf("c-peer: server OK, received id=%u x=%g y=%g\n", (unsigned)received.Id,
                   (double)received.X, (double)received.Y);
            fmx_server_destroy(server);
            return;
        }

        nap();
    }

    fprintf(stderr, "c-peer: server timed out waiting for the client\n");
    exit(1);
}

static void run_client(const char *host, uint16_t port, int udp) {
    fmx_transport transport;
    fmx_connection *connection;
    fmx_result result;
    uint64_t deadline;

    result = udp ? fmx_udp_connect(host, port, &transport) : fmx_tcp_connect(host, port, &transport);
    if (result != FMX_OK) {
        fprintf(stderr, "c-peer: connect to the interop address: %s\n", fmx_result_name(result));
        exit(1);
    }
    connection = fmx_connection_create(transport, peer_schema(), NULL, fmx_now_ms());
    if (connection == NULL) {
        fprintf(stderr, "c-peer: cannot create the connection\n");
        exit(1);
    }
    printf("c-peer: client connected to %s:%u\n", host, (unsigned)port);

    deadline = fmx_now_ms() + TIMEOUT_MS;
    while (fmx_now_ms() < deadline) {
        const fmx_event *events;
        size_t count, index;
        int have_received = 0;
        struct Player received;

        fmx_connection_tick(connection, fmx_now_ms());
        events = fmx_connection_events(connection, &count);
        for (index = 0; index < count; ++index) {
            const fmx_event *event = &events[index];
            switch (event->kind) {
            case FMX_EVENT_MESSAGE:
                if (event->message_id == PlayerEdgeCodec_MESSAGE_ID) {
                    decode_player(event->payload, event->payload_len, &received);
                    have_received = 1;
                }
                break;
            case FMX_EVENT_HANDSHAKE_FAILED:
                fprintf(stderr, "c-peer: client handshake refused: %s\n",
                        fmx_handshake_failure_name((fmx_handshake_failure)event->reason));
                exit(1);
            default:
                break;
            }
        }

        if (have_received) {
            FomoxaWriter writer;
            uint64_t drain_until;

            if (!same_player(&received, &SERVER_SENDS)) {
                fprintf(stderr, "c-peer: client got unexpected value id=%u x=%g y=%g\n",
                        (unsigned)received.Id, (double)received.X, (double)received.Y);
                exit(1);
            }
            printf("c-peer: client OK, received id=%u x=%g y=%g, replying\n",
                   (unsigned)received.Id, (double)received.X, (double)received.Y);

            encode_player(&CLIENT_SENDS, &writer);
            (void)fmx_connection_send(connection, PlayerEdgeCodec_MESSAGE_ID, writer.data,
                                      writer.len);
            fomoxa_writer_free(&writer);

            drain_until = fmx_now_ms() + 200;
            while (fmx_now_ms() < drain_until) {
                fmx_connection_tick(connection, fmx_now_ms());
                nap();
            }
            fmx_connection_destroy(connection);
            return;
        }

        nap();
    }

    fprintf(stderr, "c-peer: client timed out waiting for the server\n");
    exit(1);
}

int main(int argc, char **argv) {
    const char *role = NULL;
    const char *addr = NULL;
    const char *transport = "tcp";
    char host[256];
    uint16_t port;
    int index;

    for (index = 1; index < argc; ++index) {
        if (strcmp(argv[index], "--role") == 0 && index + 1 < argc) {
            role = argv[++index];
        } else if (strcmp(argv[index], "--addr") == 0 && index + 1 < argc) {
            addr = argv[++index];
        } else if (strcmp(argv[index], "--transport") == 0 && index + 1 < argc) {
            transport = argv[++index];
        } else {
            fprintf(stderr, "c-peer: unknown argument: %s\n", argv[index]);
            return 2;
        }
    }
    if (role == NULL) {
        fprintf(stderr, "c-peer: --role server|client is required\n");
        return 2;
    }
    if (addr == NULL) {
        fprintf(stderr, "c-peer: --addr host:port is required\n");
        return 2;
    }
    if (strcmp(transport, "tcp") != 0 && strcmp(transport, "udp") != 0) {
        fprintf(stderr, "c-peer: unknown transport: %s (expected tcp or udp)\n", transport);
        return 2;
    }

    split_addr(addr, host, sizeof(host), &port);

    if (strcmp(role, "server") == 0) {
        run_server(host, port, strcmp(transport, "udp") == 0);
    } else if (strcmp(role, "client") == 0) {
        run_client(host, port, strcmp(transport, "udp") == 0);
    } else {
        fprintf(stderr, "c-peer: unknown role: %s (expected server or client)\n", role);
        return 2;
    }

    printf("c-peer: done\n");
    return 0;
}
