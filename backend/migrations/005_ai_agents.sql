-- AI Agent 및 Persona 관리 테이블

-- 1. 페르소나 마스터
CREATE TABLE IF NOT EXISTS personas (
    id              TEXT PRIMARY KEY,
    name            VARCHAR(100) NOT NULL UNIQUE,
    description     TEXT,
    system_prompt   TEXT NOT NULL,
    model           VARCHAR(50) NOT NULL DEFAULT 'gpt-4',
    temperature     REAL NOT NULL DEFAULT 0.7,
    max_tokens      INTEGER NOT NULL DEFAULT 4096,
    tools           TEXT NOT NULL DEFAULT '[]',
    metadata        TEXT DEFAULT '{}',
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 2. AI 에이전트
CREATE TABLE IF NOT EXISTS ai_agents (
    id              TEXT PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     TEXT,
    persona_id      TEXT NOT NULL REFERENCES personas(id),
    agent_type      VARCHAR(30) NOT NULL DEFAULT 'analyzer'
                     CHECK (agent_type IN ('analyzer','responder','investigator','reporter')),
    enabled         BOOLEAN NOT NULL DEFAULT true,
    config          TEXT NOT NULL DEFAULT '{}',
    schedule        TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      VARCHAR(100),
    updated_by      VARCHAR(100)
);

CREATE INDEX IF NOT EXISTS idx_agents_persona ON ai_agents(persona_id);
CREATE INDEX IF NOT EXISTS idx_agents_type ON ai_agents(agent_type);
CREATE INDEX IF NOT EXISTS idx_agents_enabled ON ai_agents(enabled) WHERE enabled = true;

-- 3. AI 에이전트 실행 이력
CREATE TABLE IF NOT EXISTS ai_agent_runs (
    id              TEXT PRIMARY KEY,
    agent_id        TEXT NOT NULL REFERENCES ai_agents(id),
    started_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMP,
    status          VARCHAR(20) NOT NULL DEFAULT 'running'
                     CHECK (status IN ('running','completed','failed','cancelled')),
    input           TEXT,
    output          TEXT,
    error_message   TEXT,
    token_usage     INTEGER DEFAULT 0,
    duration_ms     INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_agent ON ai_agent_runs(agent_id, started_at DESC);

-- 기본 페르소나 삽입
INSERT OR IGNORE INTO personas (id, name, description, system_prompt, model, temperature, max_tokens) VALUES
('persona-security-analyst', '보안 분석가', '보안 침해 사고를 분석하고 대응 방안을 제안하는 전문가',
 '당신은 AADS(Abnormal Access Detection System)의 보안 분석가입니다. 탐지된 이상 접근 패턴을 분석하고, 공격 기법(MITRE ATT&CK)과 연관지어 설명하며, 대응 방안을 제시합니다. 항상 한국어로 답변합니다.',
 'gpt-4', 0.3, 4096),
('persona incident-responder', '사고 대응자', '보안 사고에 즉시 대응하고 격리 조치를 수행하는 전문가',
 '당신은 AADS의 사고 대응 전문가입니다. 탐지된 보안 사고에 대해 즉시 격리, 차단, 복구 조치를 권고합니다. Severity에 따라 대응 우선순위를 정하고, 단계별 대응 절차를 제시합니다. 항상 한국어로 답변합니다.',
 'gpt-4', 0.2, 4096),
('persona-threat-hunter', '위협 헌터', '능동적으로 위협을 탐지하고 예측하는 전문가',
 '당신은 AADS의 위협 헌터입니다. 현재 및 과거 탐지 데이터를 분석하여 새로운 공격 패턴을 발견하고, 잠재적 위협을 예측합니다. 공격 체인과 IOC(Indicators of Compromise)를 식별합니다. 항상 한국어로 답변합니다.',
 'gpt-4', 0.5, 4096),
('persona-report-writer', '리포트 작성자', '보안 이벤트를 분석하여 관리자용 리포트를 작성하는 전문가',
 '당신은 AADS의 리포트 작성 전문가입니다. 탐지된 보안 이벤트를 비보안 관리자도 이해할 수 있는 명확한 리포트로 작성합니다. 핵심 요약, 영향 분석, 개선 권고사항을 포함합니다. 항상 한국어로 답변합니다.',
 'gpt-4', 0.4, 8192);
