DROP TABLE catalog;
ALTER TABLE commands DROP COLUMN approved_by;
ALTER TABLE commands DROP COLUMN preview;
ALTER TABLE commands DROP COLUMN dry_run;
ALTER TABLE sites DROP COLUMN plugins_ms;
ALTER TABLE sites DROP COLUMN plugins;
ALTER TABLE sites DROP COLUMN config_ms;
ALTER TABLE sites DROP COLUMN config;
