-- 시스템 설정 테이블 (OIDC 등 설정 관리)
CREATE TABLE IF NOT EXISTS system_settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL,
    category        VARCHAR(50) NOT NULL DEFAULT 'general',
    description     TEXT,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by      VARCHAR(100)
);

-- OIDC 기본 설정 삽입
INSERT OR IGNORE INTO system_settings (key, value, category, description) VALUES
('oidc.issuer_url', 'http://localhost:8080/realms/master', 'oidc', 'OIDC Issuer URL'),
('oidc.realm', 'master', 'oidc', 'OIDC Realm'),
('oidc.client_id', 'aads', 'oidc', 'OIDC Client ID'),
('oidc.client_secret', '', 'oidc', 'OIDC Client Secret'),
('oidc.redirect_url', 'http://localhost:3000/auth/callback', 'oidc', 'OIDC Redirect URL'),
('oidc.jwt_secret', 'change-me-in-production', 'oidc', 'JWT Secret Key');
