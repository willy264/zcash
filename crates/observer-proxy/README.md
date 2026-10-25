# Observer proxy

`observer-proxy --upstream http://127.0.0.1:9067 --listen 127.0.0.1:19067 --log .runs/leaky/observations.jsonl`

The log path must be new. Each successfully recorded request receives an ordered
`seq`, UTC `ts`, lab `session`, RPC `method`, decoded `args`, and the request's
gRPC metadata. Transaction IDs are in canonical display order (the reverse of
the protocol byte order). Addresses in repeated fields remain arrays.

All methods in the pinned `CompactTxStreamer` service are forwarded. Unary and
server-streaming responses retain upstream response metadata, trailers, and errors;
streamed messages are passed through without modifying their protobuf contents.
A one-message buffer maintains backpressure and cancels the upstream stream when
the downstream client disconnects. Requests
use independent clones of the tonic channel, so a long stream does not hold a
global upstream lock. Incoming address-stream messages are logged as additional
events with `phase: "message"`. A failed log write fails the RPC before forwarding
that request. The process binds only to loopback and allows 60 seconds per upstream
request (appropriate for the small replay fixture, not long-lived production
mempool subscriptions).

`SendTransaction` records the raw test transaction and the result of inspecting
its transparent bundle prefix for versions 1–5. Invalid or unsupported prefixes
have `has_transparent_component: null` and `parse_error`; they are never labelled
shielded. This structural inspection does not verify signatures, shielded proofs,
consensus validity, or publication on a network. Having this inspector does not
mean the demo exercises transparent fallback.

Only the wallet-facing service is exposed; Darkside's staging/admin RPCs are not
available through the observer. The fixed session label is intentional: a run
models one wallet session, and it is not an identity claim about a real user.
