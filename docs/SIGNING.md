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
nonce). `server/tests/signing.rs` and `app/test/signing_test.dart` both match them byte for byte.
Never change a vector to make a test pass: a difference means clients and server disagree.

## Sealed restore words

An app-triggered restore never sends the 24 words in plaintext. `GetRestoreKey` returns a one-time
X25519 public key that lives only in the server's memory (10 minutes, bound to the caller's signing
key). The app seals the words (`app/lib/signing.dart` `sealWords`, `server/src/restore_seal.rs`):

```
key = HKDF (SHA-256)(salt = eph_pub || server_pub, ikm = X25519(eph, server), info = "itcmdr/restore-words/v1")
aad = "itcmdr-restore-words/v1\n{key_id}\n{backup}"
ct  = XChaCha20-Poly1305(key, 24-byte nonce, words encoded as UTF8, aad)
```

`StageRestore` requires `sealed_words`. A dry run must pass first; the real restore then sends that
`dry_run_id` (30 minutes, same signing key and same backup). Plaintext `words` are refused. These
calls, and the key RPCs, accept only a signed request. Pinned bytes: `proto/app_vectors.json`.

## Where the app keeps its private key

The app's Ed25519 seed is never stored in the app preferences (from v0.13.0). It is kept in:

| OS | Store | Encrypted |
|---|---|---|
| Windows | DPAPI, current user (`%APPDATA%\itcmdr\app-key.dpapi`, entropy `itcmdr-app-key/v1`). Only the same Windows user on the same PC can decrypt it. | yes |
| Linux | Secret Service keyring via `secret-tool` (attributes `service=itcmdr`, `account=app-key`) when `secret-tool` is installed and the keyring is unlocked | yes |
| Linux fallback | `$XDG_CONFIG_HOME/itcmdr/app-key` (default `~/.config`), mode 0600. Used when there is no working keyring. Protected only by the file mode and disk encryption. The Keys screen warns about it. | no |
| macOS | login Keychain via `security` (service `itcmdr`, account `app-key`) | yes |

Migration: on first start, a key found in the old `auth.key_seed` preference is written to the store and read back. Only then is the preference overwritten with zeros and removed. If the read-back fails, the preference is kept and the app keeps working. A plaintext key file that is replaced is overwritten with random bytes before it is deleted.

A DPAPI key does not move with a Windows profile copied to another PC. On a new PC, make a new key (Keys screen) and register it, then revoke the old one.
