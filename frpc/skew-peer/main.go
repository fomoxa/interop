package main

import (
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	fomoxa "github.com/fomoxa/go"

	frpc "github.com/fomoxa/frpc-go"
	"github.com/fomoxa/frpc-go/rpcnet"

	"frpc-skew-peer/generated"
	"frpc-skew-peer/models"
)

var say = frpc.Unary("Echo.Say", generated.EchoRequestRpcMessageID, generated.EchoResponseRpcMessageID)

func main() {
	os.Exit(run(os.Args[1:]))
}

func run(args []string) int {
	switch {
	case len(args) == 2 && args[0] == "serve":
		return serve(args[1])
	case len(args) == 3 && args[0] == "say":
		return sayLoud(args[1], args[2])
	default:
		fmt.Fprintln(os.Stderr, "usage: frpc-skew-peer serve <host:port> | say <host:port> <text>")
		return 64
	}
}

func schema() *fomoxa.Schema {
	messages := make([]fomoxa.Message, 0, len(generated.FomoxaMessages))
	for _, message := range generated.FomoxaMessages {
		messages = append(messages, fomoxa.Message{ID: message.ID, Fingerprint: message.Fingerprint, Prefixes: message.Prefixes})
	}
	built, err := fomoxa.NewSchema(generated.FomoxaSchemaFingerprint, messages)
	if err != nil {
		panic(err)
	}
	return built
}

func builder() *frpc.RegistryBuilder {
	return frpc.NewRegistry(generated.FRpcVoidRpcMessageID, generated.FRpcErrorRpcMessageID)
}

func encodeRequest(text string, loud bool) []byte {
	writer := generated.NewWriter()
	generated.EchoRequestRpcCodec{}.Encode(writer, &models.EchoRequest{Text: text, Loud: loud})
	return writer.Bytes()
}

func encodeResponse(text string) []byte {
	writer := generated.NewWriter()
	generated.EchoResponseRpcCodec{}.Encode(writer, &models.EchoResponse{Text: text})
	return writer.Bytes()
}

func decodeResponse(body []byte) (string, error) {
	var value models.EchoResponse
	err := generated.EchoResponseRpcCodec{}.Decode(generated.NewReader(body), &value)
	return value.Text, err
}

func serve(address string) int {
	registry, err := builder().Serve(say, func(_ frpc.Method, body []byte) (frpc.Handler, error) {
		var value models.EchoRequest
		if err := (generated.EchoRequestRpcCodec{}).Decode(generated.NewReader(body), &value); err != nil {
			return nil, frpc.InvalidArgument(err.Error())
		}
		reply := strings.ToUpper(value.Text)
		if value.Loud {
			reply += "!"
		}
		return frpc.RespondNow(encodeResponse(reply)), nil
	}).Build()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	server, err := rpcnet.Bind(address, schema(), registry, fomoxa.Config{}, frpc.Config{})
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	done := server.Start()
	fmt.Printf("listening %s\n", server.Addr())
	_, _ = io.Copy(io.Discard, os.Stdin)
	server.Stop()
	select {
	case <-done:
	case <-time.After(2 * time.Second):
	}
	return 0
}

func sayLoud(address, text string) int {
	registry, err := builder().Declare(say).Build()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	client, err := rpcnet.Connect(address, schema(), registry, 5*time.Second, rpcnet.ClientConfig{})
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	defer client.Close()
	outcome, err := client.Invoke(generated.EchoRequestRpcMessageID, encodeRequest(text, true))
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	if outcome.Kind != frpc.OutcomeResponse {
		fmt.Fprintln(os.Stderr, outcome)
		return 1
	}
	reply, err := decodeResponse(outcome.Body)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	fmt.Println(reply)
	return 0
}
