-- v0.21.0: one-time approvals of a release (catalog sha256), and the organization's
-- "require a second approver" setting (meta org.require_second_approver; off by default).
CREATE TABLE release_approvals (
  sha256 TEXT NOT NULL,
  user_id TEXT NOT NULL,
  role TEXT NOT NULL,
  pubkey TEXT NOT NULL,
  endorsement TEXT NOT NULL,
  sig TEXT NOT NULL,
  ts_ms BIGINT NOT NULL,
  PRIMARY KEY (sha256, user_id)
);
-- Existing organizations with two or more active owners/admins worked under the v0.20 rule
-- (two people for plugin commands); keep it for them until an owner turns it off.
INSERT INTO meta (k, v)
SELECT 'org.require_second_approver', 'true'
WHERE (SELECT count(*) FROM users WHERE role IN ('owner', 'admin') AND status = 'active') >= 2
  AND NOT EXISTS (SELECT 1 FROM meta WHERE k = 'org.require_second_approver');
