# Fomoxa Interop

## 1. Purpose

This repository verifies cross-language interoperability of Fomoxa SDK runtimes at the network level.
Two operating-system processes, written in different languages, perform a handshake over a real TCP or UDP socket and exchange one message each.
`fomoxac/tests/vectors/fomoxa-vectors.json` already checks that two codecs agree on byte layout, without opening a socket.
This repository checks the runtimes: that `fomoxa-net` (Rust), `Fomoxa.Net` (C#), `github.com/fomoxa/go` (Go), the `fomoxa` library (C and C++) and `@fomoxa/net` (JavaScript) speak the wire protocol, handshake included, to one another.

## 2. Peer directory structure

Each `*-peer/` directory is a self-contained project.
Every peer:

1. Declares its own copy of the `Player` model, annotated for `fomoxac`, under `src/models/`, and commits the output of `fomoxac generate` under `src/generated/`.
   This follows the convention used by `fomoxac/tests/fixtures*`.
2. Depends on its language's Fomoxa SDK through the standard distribution mechanism of that language's ecosystem.
   No peer uses a path dependency or a checked-out sibling repository.
3. Provides a `run.sh` script that wraps the peer's build output behind a single command-line contract, described in Section 4.

## 3. Dependency resolution by language

| Language | Dependency | Mechanism |
|---|---|---|
| Rust | `fomoxa-net` | crates.io, declared in `rust-peer/Cargo.toml` |
| C# | `Fomoxa.Net` | NuGet, declared in `csharp-peer/CsharpPeer.csproj` |
| Go | `github.com/fomoxa/go` | Fetched directly from the repository with `go get` and declared in `go-peer/go.mod`, which pins a pseudo-version of one commit. Go modules have no separate package registry |
| C, C++ | `github.com/fomoxa/c` | Fetched by CMake `FetchContent` at configure time and linked as `fomoxa::fomoxa`; declared in `c-peer/CMakeLists.txt` and `cpp-peer/CMakeLists.txt`. There is no C or C++ package registry for this dependency |
| JavaScript | `@fomoxa/net` | npm, declared in `js-peer/package.json` |

The C and C++ peers each carry a local copy of `fomoxa.h` at `src/models/fomoxa.h`.
This is the annotation-only marker header that `fomoxac` reads as source text.
The installed `fomoxa` library leaves it out on purpose, as `fomoxa/CMakeLists.txt` documents, and each consumer copies it in, as `fomoxac/tests/fixtures-c` and `fixtures-cpp` do.

## 4. Peer command-line contract

Every peer executable accepts the following arguments.

```
run.sh --role server|client --addr host:port [--transport tcp|udp]
```

`--transport` defaults to `tcp`.
On startup, the server sends `Player{id:100,x:10.5,y:20.0}` once a client has completed the handshake, and waits for a reply.
The client waits for that message, validates it, and replies with `Player{id:200,x:1.0,y:2.0}`.
Each side exits with status `0` once the exchange is validated, and with a non-zero status and a diagnostic on stderr otherwise.
Neither side waits longer than 10 seconds before exiting with a non-zero status.

All six models produce the same fingerprint, `0x19D8F679A9BB419F`, for `Player.edge`, although each is an independent source file.
Each model is written in the usual style of its language, and `fomoxac`'s field-name canonicalization makes the six declarations equivalent as wire contracts.
The handshake between any pair of peers depends on that equivalence.

## 5. Building

```sh
(cd rust-peer && cargo build --release)
(cd csharp-peer && dotnet build -c Release)
(cd go-peer && go build -o go-peer .)
(cd c-peer && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build)
(cd cpp-peer && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build)
(cd js-peer && npm install)
```

## 6. Running the test matrix

```sh
./run.sh
```

`run.sh` discovers every peer directory that contains an executable `run.sh`, and runs each peer as a server against every other peer as a client, in both directions, including self-pairs, over both `tcp` and `udp`.
With six peers, this produces 72 combinations (6 × 6 × 2).
The script exits with a non-zero status if any combination fails.

## 7. Continuous integration

`.github/workflows/interop.yml` builds all six peers and runs `run.sh` on every push and pull request.
Before building, it runs `fomoxac generate --check` in each peer directory, which fails if the committed `src/generated/` tree does not match what `fomoxac generate` would currently produce from `src/models/`.

## 8. Adding a language

1. Create a new `<name>-peer/` directory containing `fomoxa.toml`, a `src/models/Player.*` file written idiomatically for the target language, and the committed output of `fomoxac generate`.
   Do not copy another peer's model file.
2. Implement the command-line contract described in Section 4, using the target language's Fomoxa SDK obtained through its normal distribution mechanism.
3. Add a `run.sh` script that executes the built binary.
4. Add the corresponding build and check steps to `.github/workflows/interop.yml`.

The root `run.sh` and the pairing logic need no change, because peers are discovered by directory.

## 9. fRPC

`frpc/` runs the same kind of check one layer up, for the fRPC implementations. It is separate from the matrix above because its peers follow a different contract, and `frpc/` has no `run.sh` of its own, so the root `run.sh` does not pick it up.

| Peer | Built from | Depends on |
|---|---|---|
| `frpc/rust-peer` | `examples/interop.rs` and `examples/demo` of frpc-rust, copied (see `SOURCE`) | `fomoxa-rpc` 0.1.0 from crates.io |
| `frpc/csharp-peer` | `examples/Demo` and `examples/Interop/Program.cs` of frpc-csharp, copied (see `SOURCE`) | `Fomoxa.Rpc` 0.1.0 and `Fomoxa.Attributes` 0.1.0 from NuGet |
| `frpc/go-peer` | `cmd/frpc-interop` of frpc-go, copied | `github.com/fomoxa/frpc-go` v0.1.0, whose `examples/demo` package it imports |
| `frpc/skew-peer` | written here, with `EchoRequest` carrying one appended field | `github.com/fomoxa/frpc-go` v0.1.0 |

Every fRPC peer accepts:

```
run.sh serve <host:port>        serve the demo methods, print "listening <address>", run until stdin closes
run.sh drive <host:port>        run the eleven demo checks against a server, print "all checks passed"
run.sh frames                   print the fixed frames scenario as hex
run.sh say <host:port> <text>   call Echo.Say once and print the reply
```

`skew-peer` accepts only `serve` and `say`, and its `say` sends the appended field.

`frpc/matrix.sh` checks that every peer prints the same frames, runs every peer's `drive` against every peer's `serve` (self-pairs included), and runs `say` in both directions between `skew-peer` and every other peer, which exercises the handshake's prefix comparison and its query round across languages.

```sh
(cd frpc/rust-peer && cargo build --release)
(cd frpc/csharp-peer && dotnet build -c Release)
(cd frpc/go-peer && go build -o frpc-go-peer .)
(cd frpc/skew-peer && go build -o frpc-skew-peer .)
frpc/matrix.sh
```

Every peer depends only on released versions, so a commit to any implementation repository changes nothing here until a version in this directory is raised. The copied sources change only when they are copied again. After raising a version, rebuild and run `frpc/matrix.sh`.
