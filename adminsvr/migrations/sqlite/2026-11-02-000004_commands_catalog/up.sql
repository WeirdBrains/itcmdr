-- v0.20.0: site config and plugin documents, command previews and approvals, the plugin catalog.
ALTER TABLE sites ADD COLUMN config BLOB;
ALTER TABLE sites ADD COLUMN config_ms BIGINT NOT NULL DEFAULT 0;
ALTER TABLE sites ADD COLUMN plugins BLOB;
ALTER TABLE sites ADD COLUMN plugins_ms BIGINT NOT NULL DEFAULT 0;
ALTER TABLE commands ADD COLUMN dry_run BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE commands ADD COLUMN preview BLOB;
ALTER TABLE commands ADD COLUMN approved_by TEXT NOT NULL DEFAULT '';
CREATE TABLE catalog (
  sha256 TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  version TEXT NOT NULL,
  size BIGINT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  uploaded_by TEXT NOT NULL,
  uploaded_ms BIGINT NOT NULL,
  body BLOB NOT NULL
);
CREATE INDEX catalog_name ON catalog (name, version);
