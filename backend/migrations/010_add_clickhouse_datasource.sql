-- Add 'clickhouse' to data_sources.type CHECK constraint.
-- SQLite does not support ALTER COLUMN CHECK, so we recreate the table.
-- Data is preserved via INSERT...SELECT.

CREATE TABLE IF NOT EXISTS data_sources_new (
    id              TEXT PRIMARY KEY,
    name            VARCHAR(100) NOT NULL UNIQUE,
    type            VARCHAR(20) NOT NULL CHECK (type IN ('elasticsearch','loki','postgresql','clickhouse')),
    config          TEXT NOT NULL,
    target          VARCHAR(200) NOT NULL,
    field_mapping   TEXT NOT NULL DEFAULT '{}',
    enabled         BOOLEAN NOT NULL DEFAULT true,
    is_primary      BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO data_sources_new
    SELECT id, name, type, config, target, field_mapping, enabled, is_primary, created_at, updated_at
    FROM data_sources;

DROP TABLE IF EXISTS data_sources;

ALTER TABLE data_sources_new RENAME TO data_sources;

CREATE INDEX IF NOT EXISTS idx_data_sources_type ON data_sources(type);
CREATE INDEX IF NOT EXISTS idx_data_sources_enabled ON data_sources(enabled) WHERE enabled = true;
