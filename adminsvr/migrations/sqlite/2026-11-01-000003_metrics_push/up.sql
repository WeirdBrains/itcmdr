-- v0.19.2: site status history (redacted numbers/enums, 30 days) and push tokens per device.
CREATE TABLE site_metrics (
  site_id TEXT NOT NULL,
  ts_ms BIGINT NOT NULL,
  body BLOB NOT NULL,
  PRIMARY KEY (site_id, ts_ms)
);
CREATE TABLE push_tokens (
  token TEXT PRIMARY KEY,
  provider TEXT NOT NULL CHECK (provider IN ('fcm','apns')),
  user_id TEXT NOT NULL REFERENCES users(id),
  device_fp TEXT NOT NULL,
  platform TEXT NOT NULL DEFAULT '',
  created_ms BIGINT NOT NULL
);
CREATE INDEX push_tokens_user ON push_tokens (user_id);
