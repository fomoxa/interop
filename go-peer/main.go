package main

import (
	"flag"
	"fmt"
	"os"
	"time"

	fomoxa "github.com/fomoxa/go"

	"go-peer/src/generated"
	"go-peer/src/models"
)

const timeout = 10 * time.Second

var serverSends = models.Player{ID: 100, X: 10.5, Y: 20.0}
var clientSends = models.Player{ID: 200, X: 1.0, Y: 2.0}

const (
	largeExtraMessages    = 300
	largeFirstID          = 0x7F000000
	largeFirstFingerprint = 0xF00D000000000000
)

func schema(large bool) *fomoxa.Schema {
	messages := make([]fomoxa.Message, len(generated.FomoxaMessages))
	for i, m := range generated.FomoxaMessages {
		messages[i] = fomoxa.Message{ID: m.ID, Fingerprint: m.Fingerprint, Prefixes: m.Prefixes}
	}
	if large {
		for i := uint32(0); i < largeExtraMessages; i++ {
			fingerprint := largeFirstFingerprint + uint64(i)
			messages = append(messages, fomoxa.Message{ID: largeFirstID + i, Fingerprint: fingerprint, Prefixes: []uint64{fingerprint}})
		}
	}
	s, err := fomoxa.NewSchema(generated.FomoxaSchemaFingerprint, messages)
	if err != nil {
		panic("fomoxac never writes a schema this rejects: " + err.Error())
	}
	return s
}

func encode(player models.Player) []byte {
	w := generated.NewWriter()
	(generated.PlayerEdgeCodec{}).Encode(w, &player)
	return w.Bytes()
}

func decode(payload []byte) models.Player {
	var value models.Player
	r := generated.NewReader(payload)
	if err := (generated.PlayerEdgeCodec{}).Decode(r, &value); err != nil {
		fmt.Fprintf(os.Stderr, "go-peer: undecodable Player.edge: %v\n", err)
		os.Exit(1)
	}
	return value
}

func samePlayer(a, b models.Player) bool {
	return a.ID == b.ID && a.X == b.X && a.Y == b.Y
}

func runServer(addr string, udp bool, large bool) {
	var server *fomoxa.Server
	var err error
	if udp {
		server, err = fomoxa.ListenUDP(addr, schema(large), fomoxa.DefaultConfig())
	} else {
		server, err = fomoxa.ListenTCP(addr, schema(large), fomoxa.DefaultConfig())
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "go-peer: bind the interop address: %v\n", err)
		os.Exit(1)
	}
	defer server.Close()
	fmt.Printf("go-peer: server listening on %s\n", addr)

	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		var readyPeer *fomoxa.PeerID
		var received *models.Player

		for _, event := range server.Tick(time.Now()) {
			switch event.Kind {
			case fomoxa.EventReady:
				fmt.Printf("go-peer: peer %d ready, sending Player.edge\n", event.Peer)
				peer := event.Peer
				readyPeer = &peer
			case fomoxa.EventMessage:
				if event.MessageID == generated.PlayerEdgeMessageID {
					player := decode(event.Payload)
					received = &player
				}
			case fomoxa.EventHandshakeFailed:
				fmt.Fprintf(os.Stderr, "go-peer: server handshake refused: %s\n", event.Verdict)
				os.Exit(1)
			}
		}

		if readyPeer != nil {
			_ = server.Send(*readyPeer, generated.PlayerEdgeMessageID, encode(serverSends))
		}

		if received != nil {
			if !samePlayer(*received, clientSends) {
				fmt.Fprintf(os.Stderr, "go-peer: server got unexpected value %+v\n", *received)
				os.Exit(1)
			}
			fmt.Printf("go-peer: server OK, received %+v\n", *received)
			return
		}

		time.Sleep(10 * time.Millisecond)
	}

	fmt.Fprintln(os.Stderr, "go-peer: server timed out waiting for the client")
	os.Exit(1)
}

func runClient(addr string, udp bool, large bool) {
	var conn *fomoxa.Conn
	var err error
	if udp {
		conn, err = fomoxa.DialUDP(addr, schema(large), fomoxa.DefaultConfig())
	} else {
		conn, err = fomoxa.DialTCP(addr, schema(large), fomoxa.DefaultConfig())
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "go-peer: connect to the interop address: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()
	fmt.Printf("go-peer: client connected to %s\n", addr)

	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		var received *models.Player

		for _, event := range conn.Tick(time.Now()) {
			switch event.Kind {
			case fomoxa.EventMessage:
				if event.MessageID == generated.PlayerEdgeMessageID {
					player := decode(event.Payload)
					received = &player
				}
			case fomoxa.EventHandshakeFailed:
				fmt.Fprintf(os.Stderr, "go-peer: client handshake refused: %s\n", event.Verdict)
				os.Exit(1)
			}
		}

		if received != nil {
			if !samePlayer(*received, serverSends) {
				fmt.Fprintf(os.Stderr, "go-peer: client got unexpected value %+v\n", *received)
				os.Exit(1)
			}
			fmt.Printf("go-peer: client OK, received %+v, replying\n", *received)
			_ = conn.Send(generated.PlayerEdgeMessageID, encode(clientSends))

			drainUntil := time.Now().Add(200 * time.Millisecond)
			for time.Now().Before(drainUntil) {
				conn.Tick(time.Now())
				time.Sleep(10 * time.Millisecond)
			}
			return
		}

		time.Sleep(10 * time.Millisecond)
	}

	fmt.Fprintln(os.Stderr, "go-peer: client timed out waiting for the server")
	os.Exit(1)
}

func main() {
	role := flag.String("role", "", "server or client")
	addr := flag.String("addr", "", "host:port")
	transport := flag.String("transport", "tcp", "tcp or udp")
	schemaMode := flag.String("schema", "base", "base or large")
	flag.Parse()

	if *role == "" {
		fmt.Fprintln(os.Stderr, "go-peer: --role server|client is required")
		os.Exit(2)
	}
	if *addr == "" {
		fmt.Fprintln(os.Stderr, "go-peer: --addr host:port is required")
		os.Exit(2)
	}
	if *transport != "tcp" && *transport != "udp" {
		fmt.Fprintf(os.Stderr, "go-peer: unknown transport: %s (expected tcp or udp)\n", *transport)
		os.Exit(2)
	}
	udp := *transport == "udp"
	if *schemaMode != "base" && *schemaMode != "large" {
		fmt.Fprintf(os.Stderr, "go-peer: unknown schema: %s (expected base or large)\n", *schemaMode)
		os.Exit(2)
	}
	large := *schemaMode == "large"

	switch *role {
	case "server":
		runServer(*addr, udp, large)
	case "client":
		runClient(*addr, udp, large)
	default:
		fmt.Fprintf(os.Stderr, "go-peer: unknown role: %s (expected server or client)\n", *role)
		os.Exit(2)
	}
	fmt.Println("go-peer: done")
}
