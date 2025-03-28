External engine (ALPHA)
=======================

Using engines running outside of the browser on https://lichess.org/analysis.

Work in progress :wrench: :hammer:
----------------------------------

* [x] Implement MVP
* [x] Complete https://github.com/lichess-org/lila/pull/10867
* [ ] Implement `safe-uci` adapter
* [ ] Build easily installable local providers

Official providers
------------------

### `remote-uci`

Reference implementation in Rust.
Cross platform command line application wrapping an UCI engine.
Secure, but not robust against denial of service.

### Minimal GUIs with bundled Stockfish for Linux, Windows and Mac

Planned

Protocol (still subject to change)
----------------------------------

### Overview

Lichess provides a reference implementation for an external engine provider.
Third parties can also implement their own engine providers.

An external engine provider is a WebSocket server. To inform the client about
the connection details, it triggers a navigation to an authorization endpoint,
where the user can confirm that their client should use the given engine
provider. The client will then open a WebSocket connection for each session
with a chess engine.

The client sends [UCI commands](https://backscattering.de/chess/uci/#gui)
as text messages over the WebSocket connection. Each command is
sent in its own WebSocket message, containing no line feeds or carriage
returns.

The provider responds as if the client were exclusively communicating with
a UCI engine, by sending
[UCI commands](https://backscattering.de/chess/uci/#engine) as individual
WebSocket messages. `copyprotection` and `registration` are not supported.

### Important considerations for providers

The most straight-forward implementation would be to forward all WebSocket
messages to a UCI engine as a thin proxy. However, some important
considerations arise that require dealing with UCI specifics and tracking
the engine state.

* :warning: With many engines, a malicious user who can execute arbitrary
  commands will be able to damage the host system, cause data loss,
  exfiltrate data, or even achieve arbitrary code execution.

  Recommendation: Use the `safe-uci` adapter as a wrapper
  around UCI engines. If possible, bind the server only on the loopback
  interface to limit the attack surface.
  Generate a strong `secret` for the engine registration and do not forget to
  check it.

* Analysis is resource intensive. Be sure to put limits on CPU and memory usage
  and inforce them, in order for your system to stay responsive.

* Network connections can be interrupted.

  Recommendation: Send pings over all WebSocket connections at intervals.
  If a client times out or disconnects, stop ongoing searches in order to
  prevent deep or infinite analysis from consuming resources indefinitely.

* Clients may open multiple connections.

  Recommendation: Manage shared access to a single engine process.
  At each point, one of the WebSocket connections has an exclusive session with
  the engine. Track the engine state and options associated with each session.

  When receiving a message (except `stop`) on a connection, an
  exclusive session is requested for that connection. In order to switch
  sessions, end any ongoing search in the previous session
  (by injecting `stop`) and wait until any outstanding engine output has been
  delivered. Then issue `ucinewgame`, to ensure the following session is clean,
  and reapply any options associated with the session.

### Register external engine

To inform the client about connection details, trigger a navigation to

```
https://lichess.org/analysis/external
```

with the following query parameters:

| name | default | example | description |
| --- | --- | --- | --- |
| `url` | *required* | `ws://localhost:9670/` | URL of the provider server. External engine registrations are stored in local storage, so this may refer to `localhost` without breaking on other devices. |
| `secret` | *required* | | A secret token that the client should include in every connection request. |
| `name` | *required* | `Stockfish 15` | Short engine or provider name to show on the client. |
| `maxThreads` | `1` | `8` | Maximum number of threads supported for `setoption name Threads ...`. Make sure to respect limits of the engine as well as the machine. |
| `maxHash` | `16` | `1024` | Maximum number of memory supported for `setoption name Hash ...` (MiB). Make sure to respect limits of the engine as well as the machine. |
| `variants` | | `chess,atomic` | Comma-separated list of variants supported by `setoption name UCI_Variant ...`, if any. |

### Accepting connections

The client will open WebSocket connections to the *url* as provided in the
registration above. It will set the following additional query parameters:

| name | description |
| --- | --- |
| `secret` | The *secret* token as provided in the registration above. The provider must check and reject connection attempts if the token does not match. |
| `session` | Each new tab or session will have a different identifier. Reconnections will reuse the identifier. |

### Engine requirements

To properly work on the Lichess analysis board, engines must support:

* `UCI_Chess960`
* `MultiPV`
* `info` with
  - `depth` (reaching 6 must be fast)
  - `multipv`
  - `score`
  - `nodes` (with order of magnitude comparable to Stockfish)
  - `time`
  - `pv`
