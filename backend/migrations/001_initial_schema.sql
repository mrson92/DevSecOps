-- AADS Database Schema
-- SQLite -> PostgreSQL 호환

-- 1. 룰 마스터
CREATE TABLE IF NOT EXISTS rules (
    id              TEXT PRIMARY KEY,
    name            VARCHAR(200) NOT NULL,
    description     TEXT,
    severity        VARCHAR(20) NOT NULL CHECK (severity IN ('critical','high','medium','low','info')),
    enabled         BOOLEAN NOT NULL DEFAULT true,
    rule_type       VARCHAR(30) NOT NULL CHECK (rule_type IN ('threshold','pattern','sequence','composite')),
    condition       TEXT NOT NULL,
    window_sec      INTEGER NOT NULL DEFAULT 300,
    slide_sec       INTEGER NOT NULL DEFAULT 60,
    group_by        TEXT NOT NULL DEFAULT '[]',
    actions         TEXT NOT NULL DEFAULT '[]',
    mitre_tactics   TEXT DEFAULT '[]',
    mitre_techniques TEXT DEFAULT '[]',
    "references"      TEXT DEFAULT '[]',
    version         INTEGER NOT NULL DEFAULT 1,
    parent_rule_id  TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      VARCHAR(100),
    updated_by      VARCHAR(100)
);

CREATE INDEX IF NOT EXISTS idx_rules_enabled ON rules(enabled) WHERE enabled = true;
CREATE INDEX IF NOT EXISTS idx_rules_type ON rules(rule_type);
CREATE INDEX IF NOT EXISTS idx_rules_severity ON rules(severity);

-- 2. 룰 실행/탐지 이력
CREATE TABLE IF NOT EXISTS rule_executions (
    id              TEXT PRIMARY KEY,
    rule_id         TEXT NOT NULL REFERENCES rules(id),
    rule_version    INTEGER NOT NULL,
    detected_at     TIMESTAMP NOT NULL,
    window_start    TIMESTAMP NOT NULL,
    window_end      TIMESTAMP NOT NULL,
    matched_count   INTEGER NOT NULL DEFAULT 0,
    group_key       TEXT,
    context         TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'open'
                     CHECK (status IN ('open','acknowledged','investigating','resolved','false_positive','suppressed')),
    assignee        VARCHAR(100),
    acknowledged_at TIMESTAMP,
    resolved_at     TIMESTAMP,
    resolution_note TEXT,
    notifications   TEXT DEFAULT '[]',
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_exec_rule_time ON rule_executions(rule_id, detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_exec_status ON rule_executions(status) WHERE status IN ('open','acknowledged','investigating');
CREATE INDEX IF NOT EXISTS idx_exec_detected ON rule_executions(detected_at DESC);

-- 3. 룰 테스트/백테스트 이력
CREATE TABLE IF NOT EXISTS rule_tests (
    id              TEXT PRIMARY KEY,
    rule_id         TEXT REFERENCES rules(id),
    rule_snapshot   TEXT NOT NULL,
    test_type       VARCHAR(20) NOT NULL CHECK (test_type IN ('dry_run','backtest','sample')),
    time_range_start TIMESTAMP,
    time_range_end   TIMESTAMP,
    sample_logs     TEXT,
    result          TEXT NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','running','completed','failed')),
    error_message   TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMP
);

-- 4. 리포트
CREATE TABLE IF NOT EXISTS reports (
    id              TEXT PRIMARY KEY,
    type            VARCHAR(20) NOT NULL CHECK (type IN ('daily','weekly','monthly','custom')),
    title           VARCHAR(500) NOT NULL,
    period_start    TIMESTAMP NOT NULL,
    period_end      TIMESTAMP NOT NULL,
    content         TEXT NOT NULL,
    summary         TEXT,
    format          VARCHAR(20) NOT NULL DEFAULT 'json' CHECK (format IN ('json','html','pdf')),
    file_path       VARCHAR(500),
    status          VARCHAR(20) NOT NULL DEFAULT 'generating'
                    CHECK (status IN ('generating','completed','failed')),
    error_message   TEXT,
    generated_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reports_type_period ON reports(type, period_start DESC);

-- 5. 데이터소스 설정
CREATE TABLE IF NOT EXISTS data_sources (
    id              TEXT PRIMARY KEY,
    name            VARCHAR(100) NOT NULL UNIQUE,
    type            VARCHAR(20) NOT NULL CHECK (type IN ('elasticsearch','loki','postgresql')),
    config          TEXT NOT NULL,
    target          VARCHAR(200) NOT NULL,
    field_mapping   TEXT NOT NULL DEFAULT '{}',
    enabled         BOOLEAN NOT NULL DEFAULT true,
    is_primary      BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 6. 알림 채널
CREATE TABLE IF NOT EXISTS notification_channels (
    id              TEXT PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    type            VARCHAR(30) NOT NULL CHECK (type IN ('webhook','email','dashboard')),
    config          TEXT NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT true,
    severity_filter TEXT DEFAULT '[]',
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 7. 사용자/권한 (Keycloak 연동용 최소 정보)
CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    username        VARCHAR(100) NOT NULL UNIQUE,
    email           VARCHAR(200),
    display_name    VARCHAR(200),
    roles           TEXT NOT NULL DEFAULT '[]',
    preferences     TEXT DEFAULT '{}',
    last_login_at   TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
