import {
    buildSchema,
    connect,
    connectUdpSession,
    listen,
    listenUdpServer,
    nowMs,
} from "@fomoxa/net";

import { Player } from "./models/player.js";
import { Reader, Writer } from "./generated/runtime.js";
import { PlayerEdgeCodec } from "./generated/player_edge.js";
import { FOMOXA_MESSAGES, FOMOXA_SCHEMA_FINGERPRINT, PLAYER_EDGE_MESSAGE_ID } from "./generated/handshake.js";

const TIMEOUT_MS = 10_000;

const SERVER_SENDS = { id: 100, x: 10.5, y: 20.0 };
const CLIENT_SENDS = { id: 200, x: 1.0, y: 2.0 };

function schema() {
    return buildSchema(
        FOMOXA_SCHEMA_FINGERPRINT,
        FOMOXA_MESSAGES.map((message) => ({
            id: message.id,
            fingerprint: message.fingerprint,
            prefixes: message.prefixes,
        })),
    );
}

function encode(player) {
    const writer = new Writer();
    PlayerEdgeCodec.encode(writer, player);
    return writer.toUint8Array();
}

function decode(bytes) {
    const value = new Player();
    PlayerEdgeCodec.decode(new Reader(bytes), value);
    return value;
}

function playersEqual(a, b) {
    return a.id === b.id && a.x === b.x && a.y === b.y;
}

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function parseArgs(argv) {
    let role;
    let addr;
    let transport = "tcp";
    for (let i = 0; i < argv.length; i += 1) {
        const arg = argv[i];
        if (arg === "--role") {
            role = argv[(i += 1)];
        } else if (arg === "--addr") {
            addr = argv[(i += 1)];
        } else if (arg === "--transport") {
            transport = argv[(i += 1)];
            if (transport === undefined) {
                console.error("js-peer: --transport needs a value");
                process.exit(2);
            }
        } else {
            console.error(`js-peer: unknown argument: ${arg}`);
            process.exit(2);
        }
    }
    if (role === undefined) {
        console.error("js-peer: --role server|client is required");
        process.exit(2);
    }
    if (addr === undefined) {
        console.error("js-peer: --addr host:port is required");
        process.exit(2);
    }
    return { role, addr, transport };
}

function splitAddr(addr) {
    const index = addr.lastIndexOf(":");
    return { host: addr.slice(0, index), port: Number(addr.slice(index + 1)) };
}

async function runServer(addr, server) {
    console.log(`js-peer: server listening on ${addr}`);

    const deadline = Date.now() + TIMEOUT_MS;
    while (Date.now() < deadline) {
        let readyPeer;
        let received;

        for (const event of server.tick(nowMs())) {
            if (event.kind === "ready") {
                console.log(`js-peer: peer ${event.peer} ready, sending Player.edge`);
                readyPeer = event.peer;
            } else if (event.kind === "message" && event.messageId === PLAYER_EDGE_MESSAGE_ID) {
                received = decode(event.payload);
            } else if (event.kind === "handshake-failed") {
                console.error(`js-peer: server handshake refused: ${event.reason}`);
                process.exit(1);
            }
        }

        if (readyPeer !== undefined) {
            server.send(readyPeer, PLAYER_EDGE_MESSAGE_ID, encode(SERVER_SENDS));
        }

        if (received !== undefined) {
            if (!playersEqual(received, CLIENT_SENDS)) {
                console.error(`js-peer: server got unexpected value ${JSON.stringify(received)}`);
                process.exit(1);
            }
            console.log(`js-peer: server OK, received ${JSON.stringify(received)}`);
            server.close();
            return;
        }

        await sleep(10);
    }

    console.error("js-peer: server timed out waiting for the client");
    process.exit(1);
}

async function runClient(addr, connection) {
    console.log(`js-peer: client connected to ${addr}`);

    const deadline = Date.now() + TIMEOUT_MS;
    while (Date.now() < deadline) {
        let received;

        for (const event of connection.tick(nowMs())) {
            if (event.kind === "message" && event.messageId === PLAYER_EDGE_MESSAGE_ID) {
                received = decode(event.payload);
            } else if (event.kind === "handshake-failed") {
                console.error(`js-peer: client handshake refused: ${event.reason}`);
                process.exit(1);
            }
        }

        if (received !== undefined) {
            if (!playersEqual(received, SERVER_SENDS)) {
                console.error(`js-peer: client got unexpected value ${JSON.stringify(received)}`);
                process.exit(1);
            }
            console.log(`js-peer: client OK, received ${JSON.stringify(received)}, replying`);
            connection.send(PLAYER_EDGE_MESSAGE_ID, encode(CLIENT_SENDS));
            await sleep(100);
            connection.tick(nowMs());
            connection.destroy();
            return;
        }

        await sleep(10);
    }

    console.error("js-peer: client timed out waiting for the server");
    process.exit(1);
}

async function main() {
    const args = parseArgs(process.argv.slice(2));
    const { host, port } = splitAddr(args.addr);

    if (args.role === "server" && args.transport === "tcp") {
        await runServer(args.addr, await listen(host, port, schema()));
    } else if (args.role === "server" && args.transport === "udp") {
        await runServer(args.addr, await listenUdpServer(host, port, schema()));
    } else if (args.role === "client" && args.transport === "tcp") {
        await runClient(args.addr, await connect(host, port, schema()));
    } else if (args.role === "client" && args.transport === "udp") {
        await runClient(args.addr, await connectUdpSession(host, port, schema()));
    } else {
        console.error(
            `js-peer: unsupported combination: --role ${args.role} --transport ${args.transport} ` +
                "(role is server|client, transport is tcp|udp)",
        );
        process.exit(2);
    }

    console.log("js-peer: done");
}

main();
