# Fomoxa Interop

## 1. Purpose

This repository verifies cross-language interoperability of Fomoxa SDK runtimes at the network level.
Two operating-system processes, written in different languages, perform a handshake over a real TCP or UDP socket and exchange one message each.
This is distinct from the check performed by `fomoxac/tests/vectors/fomoxa-vectors.json`, which verifies that two codecs agree on byte layout without opening a socket.
This repository verifies that the *runtimes* — `fomoxa-net` (Rust), `Fomoxa.Net` (C#), `github.com/fomoxa/go` (Go), and the `fomoxa` library (C and C++) — speak the wire protocol, including the handshake, to one another.

## 2. Peer Directory Structure

Each `*-peer/` directory is a self-contained project.
Every peer:

1. Declares its own copy of the `Player` model, annotated for `fomoxac`, under `src/models/`, and commits the output of `fomoxac generate` under `src/generated/`.
   This follows the convention used by `fomoxac/tests/fixtures*`.
2. Depends on its language's Fomoxa SDK through the standard distribution mechanism of that language's ecosystem.
   No peer uses a path dependency or a checked-out sibling repository.
3. Provides a `run.sh` script that wraps the peer's build output behind a single command-line contract, described in Section 4.

## 3. Dependency Resolution by Language

| Language | Dependency | Mechanism |
|---|---|---|
| Rust | `fomoxa-net` | crates.io, declared in `rust-peer/Cargo.toml` |
| C# | `Fomoxa.Net` | NuGet, declared in `csharp-peer/CsharpPeer.csproj` |
| Go | `github.com/fomoxa/go` | Fetched directly from the repository via `go get`; declared in `go-peer/go.mod`. Go modules have no separate package registry; `go get` resolves a pseudo-version from the repository's commit history |
| C, C++ | `github.com/fomoxa/c` | Fetched by CMake `FetchContent` at configure time and linked as `fomoxa::fomoxa`; declared in `c-peer/CMakeLists.txt` and `cpp-peer/CMakeLists.txt`. There is no C or C++ package registry for this dependency |

The C and C++ peers each carry a local copy of `fomoxa.h` at `src/models/fomoxa.h`.
This is the annotation-only marker header that `fomoxac` reads as source text.
It is intentionally excluded from the installed `fomoxa` library, as documented in `fomoxa/CMakeLists.txt`; each consumer of the library copies it in, the same way `fomoxac/tests/fixtures-c` and `fixtures-cpp` do.

## 4. Peer Command-Line Contract

Every peer executable accepts the following arguments.

```
run.sh --role server|client --addr host:port [--transport tcp|udp]
```

`--transport` defaults to `tcp`.
On startup, the server sends `Player{id:100,x:10.5,y:20.0}` once a client has completed the handshake, and waits for a reply.
The client waits for that message, validates it, and replies with `Player{id:200,x:1.0,y:2.0}`.
Each side exits with status `0` once the exchange is validated, and with a non-zero status and a diagnostic on stderr otherwise.
Neither side waits longer than 10 seconds before exiting with a non-zero status.

All five models produce the identical fingerprint `0x19D8F679A9BB419F` for `Player.edge`, despite being independent source files rather than copies of one another.
Each model is written in the idiomatic style of its language; `fomoxac`'s field-name canonicalization is what makes the five declarations equivalent at the wire-contract level.
This equivalence is a precondition for the handshake to succeed between any pair of peers.

## 5. Building

```sh
(cd rust-peer && cargo build --release)
(cd csharp-peer && dotnet build -c Release)
(cd go-peer && go build -o go-peer .)
(cd c-peer && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build)
(cd cpp-peer && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build)
```

## 6. Running the Test Matrix

```sh
./run.sh
```

`run.sh` discovers every peer directory that contains an executable `run.sh`, and runs each peer as a server against every other peer as a client, in both directions, including self-pairs, over both `tcp` and `udp`.
With five peers, this produces 50 combinations (5 × 5 × 2).
The script exits with a non-zero status if any combination fails.

## 7. Continuous Integration

`.github/workflows/interop.yml` builds all five peers and runs `run.sh` on every push and pull request.
Before building, it runs `fomoxac generate --check` in each peer directory, which fails if the committed `src/generated/` tree does not match what `fomoxac generate` would currently produce from `src/models/`.

## 8. Adding a Language

1. Create a new `<name>-peer/` directory containing `fomoxa.toml`, a `src/models/Player.*` file written idiomatically for the target language, and the committed output of `fomoxac generate`.
   Do not copy another peer's model file.
2. Implement the command-line contract described in Section 4, using the target language's Fomoxa SDK obtained through its normal distribution mechanism.
3. Add a `run.sh` script that executes the built binary.
4. Add the corresponding build and check steps to `.github/workflows/interop.yml`.

No change to the root `run.sh` or to the pairing logic is required; peers are discovered by directory.
