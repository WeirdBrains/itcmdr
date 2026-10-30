# Signed requests: canonical encoding

Every signed gRPC call carries `x-itcmdr-key` (key fingerprint), `x-itcmdr-ts` (unix ms),
`x-itcmdr-nonce` (16-128 chars of `[A-Za-z0-9_-]`, never reused) and `x-itcmdr-sig` (hex Ed25519).
The signature covers:

```
itcmdr-req/v1\n{gRPC path}\n{ts}\n{nonce}\n{sha256 hex of the request body}
```

The server does not see the client's raw bytes: it hashes its own protobuf **re-encoding** of the
decoded request. A client therefore has to produce the canonical encoding, which prost (Rust) and the
official Dart `protobuf` package both do by default:

1. Fields in ascending field-number order.
2. Proto3 default values are omitted: `0`, `false`, `""`, empty repeated fields, unset messages.
   A set sub-message is written even when all its fields are default.
3. Repeated scalars (none in the API today) are packed; repeated strings are one entry per item.
4. Strings are UTF8 bytes, not Unicode-normalized; no unknown fields; no duplicate fields; minimal varints.
5. Negative `int32`/`int64` are 10-byte two's-complement varints (not zigzag).

Pinned vectors: `proto/signing_vectors.json` (body hex plus the exact signed message for a fixed ts and
nonce). `server/tests/signing.rs` checks Rust against them; the app's test must match them byte for
byte when it starts signing (v0.12.0). Never change a vector to make a test pass: a difference means
clients and server disagree.
