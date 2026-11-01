-- v0.19.1 passkeys / security keys (WebAuthn second factor).
CREATE TABLE passkeys (
  credential_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  label TEXT NOT NULL,
  alg BIGINT NOT NULL,
  public_key BLOB NOT NULL,
  aaguid TEXT NOT NULL,
  attestation TEXT NOT NULL,
  sign_count BIGINT NOT NULL DEFAULT 0,
  hmac_secret BOOLEAN NOT NULL DEFAULT 0,
  created_ms BIGINT NOT NULL,
  last_used_ms BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX passkeys_user ON passkeys (user_id);
