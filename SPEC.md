# Abnormal Access Detection System (AADS)

## 1. 문서 정보

| 항목 | 내용 |
|------|------|
| **문서명** | AADS_SPEC_v1.0.md |
| **버전** | 1.0 |
| **작성일** | 2026-08-23 |
| **상태** | 초안 (리뷰 필요) |
| **목적** | 로그 표준화 기반 Access/Application Log에서 이상 접근 탐지·분석·리포팅 |

---

## 2. 시스템 개요

### 2.1 목적
로그 표준화 문서(DAOU-PF-OP-002, 아파치로그표준화예시) 기반으로 수집된 Access Log 및 Application Log에서 **이상 접근(Abnormal Access)**을 Rule-based로 탐지·분석·리포팅하는 시스템 구축

### 2.2 범위
| 포함 | 제외 |
|------|------|
| ElasticSearch 연동 쿼리/탐지 엔진 | 로그 수집 에이전트 구축 (Fluent Bit/Vector 기구축) |
| SQLite/PostgreSQL 룰 저장소 | ML 기반 이상탐지 (2차) |
| React+Vite 대시보드 (Shadcn/ui) | SIEM/SOAR 자동대응 (웹훅만 제공) |
| OIDC 인증 (Keycloak 연동) | 다중 테넌트 (단일 테넌트 MVP) |
| 주기 리포트 생성 (일/주/월) | 실시간 스트리밍 처리 (배치 윈도우 기반) |

### 2.3 핵심 지표 (KPI)
| 지표 | 목표 |
|------|------|
| 룰 평가 지연시간 (1만 로그 기준) | < 1초 |
| 탐지 정확도 (False Positive) | < 5% |
| 대시보드 초기 로드 | < 2초 |
| API P99 응답시간 | < 500ms |
| 가용성 | 99.5% (단일 서버) |

---

## 3. 기술 스택

### 3.1 백엔드 (Rust)
| 영역 | 선택 | 근거 |
|------|------|------|
| **Web Framework** | Axum | 타입 안전성, Tower 에코시스템, 성능 |
| **Async Runtime** | Tokio | 표준, 생태계 성숙 |
| **DB ORM** | SQLx | 컴파일타임 쿼리 검증, 비동기 네이티브 |
| **ES Client** | `elasticsearch-rs` | 공식 클라이언트, 비동기 지원 |
| **Auth** | `oauth2` + `jsonwebtoken` | OIDC 표준 준수 |
| **Config** | `config-rs` + `figment` | 환경별 설정 관리 |
| **Observability** | `tracing` + `opentelemetry` | 구조화 로깅, 메트릭, 트레이싱 |
| **Serialization** | `serde` + `serde_json` | 표준 |
| **Validation** | `validator` | 룰/입력 검증 |
| **Scheduler** | `tokio-cron-scheduler` | 주기적 룰 평가, 리포트 생성 |

**의존성 최소화 전략**: `default-features = false`로 불필요한 피처 비활성화, `cargo-deny`로 라이선스/보안 검사

### 3.2 프론트엔드
| 영역 | 선택 | 근거 |
|------|------|------|
| **Framework** | React 18 + Vite | 빌드 속도, SPA |
| **Language** | TypeScript | 타입 안전성 |
| **UI** | Shadcn/ui + Tailwind CSS | 일관성, 커스터마이징 용이 |
| **상태 관리** | TanStack Query (서버 상태), Zustand (클라이언트) | 캐싱, 실시간 |
| **폼** | React Hook Form + Zod | 룰 생성/수정 검증 |
| **차트** | Recharts | 라이트, 커스텀 용이 |
| **에디터** | Monaco/CodeMirror | CEL/DSL 구문 강조 |

### 3.3 인프라
| 영역 | 선택 | 근거 |
|------|------|------|
| **배포** | Docker Compose (단일 서버) | MVP 운영 편의성 |
| **리버스 프록시** | Nginx | SSL 종료, 정적 파일 서빙 |
| **로그 스토리지** | ElasticSearch 8.x | 풀텍스트 검색, 집계 |
| **룰 저장소** | SQLite (MVP) -> PostgreSQL (확장) | 경량, 이식성 |
| **인증** | Keycloak (OIDC) | SSO 연동 |

---

## 4. 아키텍처 상세

### 4.1 컴포넌트 다이어그램

```
+---------------------------------------------------------------------+
|                         EXTERNAL SYSTEMS                            |
|  +--------------+  +--------------+  +--------------+                |
|  | Fluent Bit/  |  |  Keycloak    |  |  Slack/      |                |
|  | Vector       |  |  (OIDC)      |  |  Teams       |                |
|  +------+-------+  +------+-------+  +------+-------+                |
+--------|-----------------|-----------------|-------------------------+
         |                 |                 |
         v                 v                 v
+---------------------------------------------------------------------+
|                         AADS BACKEND (Rust/Axum)                    |
|  +-------------+ +-------------+ +-------------+ +-------------+   |
|  | Auth Module | | Rule Engine | | ES Query    | | Scheduler   |   |
|  | (OIDC/JWT)  | | (CEL/DSL)   | | Builder     | | (Cron)      |   |
|  +------+------+ +------+------+ +------+------+ +------+------+   |
|         |               |               |               |           |
|         +---------------+---------------+---------------+           |
|                         v               v                          |
|              +----------------------------------+                  |
|      |       |      Data Access Layer          |                   |
|       |  +---------+  +--------------+  |                   |
|       |  | SQLite  |  | ElasticSearch|  |                   |
|       |  | (Rules) |  | (Logs)       |  |                   |
|       |  +---------+  +--------------+  |                   |
|       +----------------------------------+                  |
+---------------------------------------------------------------------+
         |
         v
+---------------------------------------------------------------------+
|                        FRONTEND (React+Vite)                        |
|  +------------+ +------------+ +------------+ +------------+        |
|  | Dashboard  | | Rule Mgmt  | | Detections | | Reports    |        |
|  | (Realtime) | | (CRUD/Test)| | (Timeline) | | (Schedule) |        |
|  +------------+ +------------+ +------------+ +------------+        |
+---------------------------------------------------------------------+
```

### 4.2 데이터 플로우

```
[Log Ingestion] -> [ElasticSearch] -> [Scheduler: 1분마다] -> [Rule Engine]
                                                                    |
                                    +-------------------------------+
                                    v
                            [Match Found?] --No--> [Next Rule]
                                    |Yes
                                    v
                            [Deduplication] -> [Rule Execution Record]
                                    |
                                    v
                            [Alert/Notification] -> [Dashboard WS Push]
                                    |
                                    v
                            [Report Aggregation] -> [Daily/Weekly/Monthly Report]
```

---

## 5. 데이터 모델

### 5.1 ElasticSearch 인덱스 템플릿 (로그 표준화 반영)

```json
{
  "index_patterns": ["access-logs-*", "app-logs-*"],
  "priority": 100,
  "template": {
    "settings": {
      "index": {
        "number_of_shards": 3,
        "number_of_replicas": 1,
        "refresh_interval": "10s",
        "codec": "best_compression"
      }
    },
    "mappings": {
      "properties": {
        "@timestamp": { "type": "date", "format": "strict_date_optional_time||epoch_millis" },
        "log": {
          "properties": {
            "type": { "type": "keyword" },
            "level": { "type": "keyword" },
            "logger": { "type": "keyword" },
            "message": { "type": "text", "analyzer": "korean" },
            "original": { "type": "text", "index": false }
          }
        },
        "network": {
          "properties": {
            "client": {
              "properties": {
                "ip": { "type": "ip" },
                "port": { "type": "integer" },
                "geo": {
                  "properties": {
                    "country": { "type": "keyword" },
                    "city": { "type": "keyword" },
                    "location": { "type": "geo_point" },
                    "asn": { "type": "keyword" }
                  }
                }
              }
            },
            "server": {
              "properties": {
                "ip": { "type": "ip" },
                "port": { "type": "integer" },
                "domain": { "type": "keyword" }
              }
            },
            "direction": { "type": "keyword" },
            "protocol": { "type": "keyword" },
            "transport": { "type": "keyword" }
          }
        },
        "http": {
          "properties": {
            "request": {
              "properties": {
                "method": { "type": "keyword" },
                "scheme": { "type": "keyword" },
                "host": { "type": "keyword" },
                "path": { "type": "keyword" },
                "query": { "type": "text", "analyzer": "simple" },
                "version": { "type": "keyword" },
                "headers": { "type": "object", "enabled": true },
                "body": { "type": "text", "index": false },
                "size": { "type": "long" }
              }
            },
            "response": {
              "properties": {
                "status_code": { "type": "short" },
                "status_phrase": { "type": "keyword" },
                "headers": { "type": "object", "enabled": true },
                "body": { "type": "text", "index": false },
                "size": { "type": "long" },
                "latency_ms": { "type": "integer" }
              }
            },
            "user_agent": {
              "properties": {
                "original": { "type": "keyword" },
                "browser": { "type": "keyword" },
                "os": { "type": "keyword" },
                "device": { "type": "keyword" },
                "is_bot": { "type": "boolean" }
              }
            }
          }
        },
        "app": {
          "properties": {
            "service": { "type": "keyword" },
            "instance": { "type": "keyword" },
            "trace_id": { "type": "keyword" },
            "span_id": { "type": "keyword" },
            "user": {
              "properties": {
                "id": { "type": "keyword" },
                "name": { "type": "keyword" },
                "roles": { "type": "keyword" },
                "session_id": { "type": "keyword" }
              }
            },
            "business": {
              "properties": {
                "action": { "type": "keyword" },
                "resource": { "type": "keyword" },
                "result": { "type": "keyword" },
                "risk_score": { "type": "float" }
              }
            }
          }
        },
        "tags": { "type": "keyword" },
        "mitre_techniques": { "type": "keyword" }
      }
    }
  }
}
```

### 5.2 룰 스토리지 스키마 (SQLite -> PostgreSQL 호환)

```sql
-- 1. 룰 마스터
CREATE TABLE rules (
    id              CHAR(36) PRIMARY KEY,
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
    references      TEXT DEFAULT '[]',
    version         INTEGER NOT NULL DEFAULT 1,
    parent_rule_id  CHAR(36),
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      VARCHAR(100),
    updated_by      VARCHAR(100)
);

CREATE INDEX idx_rules_enabled ON rules(enabled) WHERE enabled = true;
CREATE INDEX idx_rules_type ON rules(rule_type);
CREATE INDEX idx_rules_severity ON rules(severity);

-- 2. 룰 실행/탐지 이력
CREATE TABLE rule_executions (
    id              CHAR(36) PRIMARY KEY,
    rule_id         CHAR(36) NOT NULL REFERENCES rules(id),
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

CREATE INDEX idx_exec_rule_time ON rule_executions(rule_id, detected_at DESC);
CREATE INDEX idx_exec_status ON rule_executions(status) WHERE status IN ('open','acknowledged','investigating');
CREATE INDEX idx_exec_detected ON rule_executions(detected_at DESC);

-- 3. 룰 테스트/백테스트 이력
CREATE TABLE rule_tests (
    id              CHAR(36) PRIMARY KEY,
    rule_id         CHAR(36) REFERENCES rules(id),
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
CREATE TABLE reports (
    id              CHAR(36) PRIMARY KEY,
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

CREATE INDEX idx_reports_type_period ON reports(type, period_start DESC);

-- 5. 데이터소스 설정
CREATE TABLE data_sources (
    id              CHAR(36) PRIMARY KEY,
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
CREATE TABLE notification_channels (
    id              CHAR(36) PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    type            VARCHAR(30) NOT NULL CHECK (type IN ('webhook','email','dashboard')),
    config          TEXT NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT true,
    severity_filter TEXT DEFAULT '[]',
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 7. 사용자/권한 (Keycloak 연동용 최소 정보)
CREATE TABLE users (
    id              CHAR(36) PRIMARY KEY,
    username        VARCHAR(100) NOT NULL UNIQUE,
    email           VARCHAR(200),
    display_name    VARCHAR(200),
    roles           TEXT NOT NULL DEFAULT '[]',
    preferences     TEXT DEFAULT '{}',
    last_login_at   TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

---

## 6. 룰 엔진 상세

### 6.1 룰 타입 정의

```rust
pub enum RuleType {
    Threshold,   // 임계값: "5분 내 404 > 100회"
    Pattern,     // 패턴 매칭: "SQLi 시그니처 매칭"
    Sequence,    // 시퀀스: "로그인 실패 5회 -> 성공 -> 권한 상승 시도"
    Composite,   // 복합: AND/OR/NOT 조합
}

pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

pub struct RuleCondition {
    pub expression: String,       // CEL 표현식 또는 DSL
    pub window_sec: u32,          // 평가 윈도우
    pub group_by: Vec<String>,    // 그룹핑 키
    pub having: Option<String>,   // 집계 후 필터
}
```

### 6.2 CEL 표현식 예시

```cel
// Threshold: 5분 내 동일 IP에서 4xx 응답 50회 초과
size(filter(logs, log -> log.http.response.status_code >= 400 && log.http.response.status_code < 500)) > 50

// Pattern: SQL Injection 시그니처
exists(logs, log -> log.http.request.query.matches("(?i)(union|select|insert|update|delete|drop|exec).*"))

// Composite: 비정상 시간대 관리자 접근
network.client.ip NOT IN ['10.0.0.0/8'] && app.user.roles CONTAINS 'admin' && hour(@timestamp) < 6
```

### 6.3 룰 평가 파이프라인

```rust
async fn evaluate_rule(rule: &Rule, es: &EsClient, store: &RuleStore) -> Result<Vec<Detection>> {
    let window = TimeWindow::sliding(rule.window_sec, rule.slide_sec);
    let query = QueryBuilder::build(&rule.condition, &window, &rule.group_by);
    let buckets = es.aggregate(query).await?;

    let mut detections = Vec::new();
    for bucket in buckets {
        if rule.condition.evaluate(&bucket)? {
            if !store.is_duplicate(&rule.id, &bucket.group_key, Duration::hours(1)).await? {
                detections.push(Detection::from_bucket(&rule, bucket));
            }
        }
    }

    store.save_executions(&detections).await?;

    for det in &detections {
        notification::dispatch(&rule.actions, det).await;
    }

    Ok(detections)
}
```

### 6.4 초기 룰셋 (MVP 10개)

| # | 룰명 | 타입 | 심각도 | 설명 | MITRE |
|---|------|------|--------|------|-------|
| 1 | Brute Force - Login | threshold | high | 5분 내 동일 IP 로그인 실패 10회 | T1110 |
| 2 | SQL Injection Attempt | pattern | critical | 쿼리스트링 SQL 키워드 매칭 | T1190 |
| 3 | XSS Attempt | pattern | high | XSS 스크립트 태그 매칭 | T1189 |
| 4 | Path Traversal | pattern | high | 경로 순회 패턴 매칭 (../../) | T1083 |
| 5 | Web Shell Access | pattern | critical | 웹쉘 파일 접근 패턴 | T1505.003 |
| 6 | Privilege Escalation | sequence | critical | 로그인후 관리자 API 접근 시도 | T1068 |
| 7 | Off-Hours Admin Access | composite | medium | 비업무시간(00-06시) 관리자 접근 | T1078 |
| 8 | Bot/Scanner Traffic | threshold | low | 1분 내 동일 IP 100회 이상 요청 | - |
| 9 | Data Exfiltration | threshold | high | 단시간 대용량 응답 (10MB+) | T1048 |
| 10 | Port Scan / Enumeration | threshold | medium | 5분 내 10개 이상 경로 접근 | T1046 |

---

## 7. API 계약서

### 7.1 인증
- **Authorization**: `Bearer <access_token>` (OIDC JWT)
- **Roles**: `admin` (전체), `analyst` (읽기/룰테스트/탐지처리), `viewer` (읽기만)

### 7.2 엔드포인트

| 리소스 | 메서드 | 경로 | 권한 | 설명 |
|--------|--------|------|------|------|
| **Rules** | GET | `/api/v1/rules` | viewer+ | 룰 목록 (페이징, 필터) |
| | POST | `/api/v1/rules` | admin | 룰 생성 |
| | GET | `/api/v1/rules/{id}` | viewer+ | 룰 상세 |
| | PUT | `/api/v1/rules/{id}` | admin | 룰 수정 (버전업) |
| | DELETE | `/api/v1/rules/{id}` | admin | 룰 비활성화/삭제 |
| | POST | `/api/v1/rules/{id}/test` | analyst+ | 룰 테스트 |
| | POST | `/api/v1/rules/{id}/clone` | analyst+ | 룰 복사 |
| **Detections** | GET | `/api/v1/detections` | viewer+ | 탐지 목록 |
| | GET | `/api/v1/detections/{id}` | viewer+ | 탐지 상세 |
| | PATCH | `/api/v1/detections/{id}` | analyst+ | 상태 변경 |
| | POST | `/api/v1/detections/bulk-action` | analyst+ | 일괄 상태 변경 |
| **Dashboard** | GET | `/api/v1/dashboard/stats` | viewer+ | 실시간 통계 |
| | GET | `/api/v1/dashboard/timeline` | viewer+ | 시계열 차트 |
| | GET | `/api/v1/dashboard/top` | viewer+ | Top N |
| | WS | `/api/v1/dashboard/ws` | viewer+ | 실시간 푸시 |
| **Reports** | GET | `/api/v1/reports` | viewer+ | 리포트 목록 |
| | POST | `/api/v1/reports` | analyst+ | 리포트 생성 |
| | GET | `/api/v1/reports/{id}` | viewer+ | 리포트 다운로드 |
| **DataSources** | GET | `/api/v1/data-sources` | admin | 데이터소스 목록 |
| | POST | `/api/v1/data-sources` | admin | 데이터소스 등록 |
| | PUT | `/api/v1/data-sources/{id}` | admin | 데이터소스 수정 |
| | POST | `/api/v1/data-sources/{id}/test` | admin | 연결 테스트 |
| **Notifications** | GET | `/api/v1/notifications/channels` | admin | 채널 목록 |
| | POST | `/api/v1/notifications/channels` | admin | 채널 등록 |
| | POST | `/api/v1/notifications/channels/{id}/test` | admin | 테스트 발송 |
| **Auth** | GET | `/api/v1/auth/me` | all | 현재 사용자 |
| | GET | `/api/v1/auth/oidc/login` | - | OIDC 로그인 |
| | GET | `/api/v1/auth/oidc/callback` | - | OIDC 콜백 |

### 7.3 공통 응답 형식

```json
// 성공
{
  "success": true,
  "data": { },
  "meta": { "page": 1, "size": 20, "total": 100 }
}

// 에러
{
  "success": false,
  "error": {
    "code": "RULE_VALIDATION_FAILED",
    "message": "CEL expression parse error",
    "details": { "position": 42, "expected": "identifier" }
  }
}
```

---

## 8. 프론트엔드 상세

### 8.1 라우팅 구조

```
/                          -> Dashboard (실시간)
/rules                     -> Rule Management
/rules/new                 -> Rule Create
/rules/:id                 -> Rule Detail (탭: 개요, 조건, 히스토리, 테스트)
/rules/:id/edit            -> Rule Edit
/detections                -> Detection List
/detections/:id            -> Detection Detail (타임라인, 로그 원문, MITRE)
/reports                   -> Report List
/reports/new               -> Report Builder
/reports/:id               -> Report View/Download
/settings                  -> Settings
/settings/data-sources     -> Data Source 관리
/settings/notifications    -> 알림 채널 관리
/settings/users            -> 사용자/권한
/profile                   -> 내 프로필
```

### 8.2 상태 관리 전략

| 상태 유형 | 도구 | 적용 범위 |
|-----------|------|-----------|
| **Server State** | TanStack Query | API 데이터, 캐싱 |
| **Client State** | Zustand | UI 상태 (사이드바, 모달) |
| **Real-time** | WebSocket + Query Invalidation | 대시보드, 알림 |
| **Forms** | React Hook Form + Zod | 룰 생성/수정 |

### 8.3 핵심 컴포넌트 (Shadcn/ui 기반)

```
components/
  ui/                          # shadcn 원본
    button.tsx
    dialog.tsx
    data-table.tsx             # TanStack Table 래퍼
    select.tsx
    tabs.tsx
    toast.tsx
    chart.tsx                  # Recharts 래퍼
  rule/
    RuleEditor.tsx             # CEL/DSL 에디터
    RuleTestPanel.tsx          # 테스트 실행/결과
    RuleVersionHistory.tsx
    RuleSeverityBadge.tsx
  detection/
    DetectionTable.tsx         # 가상화 테이블
    DetectionTimeline.tsx
    LogContextViewer.tsx       # 원본 로그 뷰어
    MitreTag.tsx
  dashboard/
    StatCard.tsx
    TrendChart.tsx
    TopNChart.tsx
    RealTimeBadge.tsx
  report/
    ReportBuilder.tsx
    ChartConfigPanel.tsx
    ExportDialog.tsx
```

---

## 9. 배포 구조

### 9.1 Docker Compose (프로덕션)

```yaml
version: '3.8'

services:
  backend:
    build:
      context: ./backend
      dockerfile: Dockerfile.prod
    image: aads-backend:${VERSION:-latest}
    environment:
      - ENV=production
      - DATABASE_URL=sqlite:///data/rules.db
      - ELASTICSEARCH_URL=http://elasticsearch:9200
      - ELASTICSEARCH_USERNAME=${ES_USER}
      - ELASTICSEARCH_PASSWORD=${ES_PASS}
      - OIDC_ISSUER=${OIDC_ISSUER}
      - OIDC_CLIENT_ID=${OIDC_CLIENT_ID}
      - OIDC_CLIENT_SECRET=${OIDC_CLIENT_SECRET}
      - JWT_SECRET=${JWT_SECRET}
      - RUST_LOG=info,aads=debug
      - TZ=Asia/Seoul
    volumes:
      - aads_data:/data
    ports:
      - "127.0.0.1:8080:8080"
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped
    depends_on:
      elasticsearch:
        condition: service_healthy

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile.prod
    image: aads-frontend:${VERSION:-latest}
    environment:
      - VITE_API_BASE_URL=https://aads.example.com/api/v1
      - VITE_WS_URL=wss://aads.example.com/api/v1/dashboard/ws
    ports:
      - "127.0.0.1:3000:80"
    depends_on: [backend]
    restart: unless-stopped

  elasticsearch:
    image: docker.elastic.co/elasticsearch/elasticsearch:8.11.0
    environment:
      - discovery.type=single-node
      - xpack.security.enabled=true
      - ELASTIC_PASSWORD=${ES_PASS}
      - ES_JAVA_OPTS=-Xms2g -Xmx2g
      - cluster.routing.allocation.disk.threshold_enabled=false
    volumes:
      - es_data:/usr/share/elasticsearch/data
    ports:
      - "127.0.0.1:9200:9200"
    deploy:
      resources:
        limits:
          memory: 4G
    healthcheck:
      test: ["CMD-SHELL", "curl -s http://localhost:9200/_cluster/health | grep -q green"]
      interval: 30s
      timeout: 10s
      retries: 5
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/certs:/etc/nginx/certs:ro
    ports:
      - "80:80"
      - "443:443"
    depends_on: [backend, frontend]
    restart: unless-stopped

volumes:
  aads_data:
  es_data:
```

### 9.2 헬스체크 엔드포인트

```json
// GET /health
{
  "status": "healthy",
  "version": "1.0.0",
  "checks": {
    "database": { "status": "up", "latency_ms": 2 },
    "elasticsearch": { "status": "up", "latency_ms": 15 },
    "scheduler": { "status": "running" }
  }
}
```

---

## 10. 개발 단계별 완료 기준

### Phase 1: 스펙 확정 & 문서화 (1주차)

| 산출물 | 완료 기준 |
|--------|-----------|
| **SPEC.md** (본 문서) | 이해관계자 리뷰 완료, 서명/승인 |
| **API 계약서** | Swagger UI에서 검증 가능, 프론트/백 동시 개발 가능 |
| **ERD & ES 매핑** | 로그 표준 문서 필드 100% 매핑표 완성 |
| **초기 룰셋 정의서** | 10개 룰 상세 조건/테스트케이스 정의 |
| **인프라 설계서** | Docker Compose, 네트워크, 시크릿 관리 문서화 |
| **개발 환경 세팅 가이드** | 신규 개발자 30분 내 로컬 실행 가능 |

**승인 게이트**: 아키텍트 + 보안팀 + 운영팀 3자 승인

---

### Phase 2: 백엔드 코어 구현 (2주차)

| 작업 | 완료 기준 |
|------|-----------|
| **프로젝트 스캐폴딩** | `cargo new`, 워크스페이스 구성, CI (lint, test, build) |
| **설정 관리** | 환경별 config (dev/staging/prod), 시크릿 외부화 |
| **데이터베이스** | SQLx 마이그레이션 (7개 테이블), `cargo sqlx prepare` 통과 |
| **ES 클라이언트** | 연결 풀, 재시도, 헬스체크, 인덱스 템플릿 자동 적용 |
| **인증 모듈** | OIDC Discovery, PKCE, JWT 검증, 역할 매핑, 토큰 갱신 |
| **미들웨어** | 요청 ID, 구조화 로깅, CORS, 레이트리밋, 에러 핸들링 |
| **헬스체크/메트릭** | `/health`, `/metrics` (Prometheus), 트레이싱 (OTel) |
| **단위 테스트** | 커버리지 >= 80% (auth, config, db, es client) |

**승인 게이트**: `cargo test --workspace --all-targets` 통과, 클린 빌드

---

### Phase 3: 룰 엔진 & ES 연동 (2주차)

| 작업 | 완료 기준 |
|------|-----------|
| **Rule CRUD API** | OpenAPI 스펙 100% 구현, 입력 검증 |
| **CEL 파서/평가기** | cel-rs 통합, 샘플 룰 10개 컴파일/실행 성공 |
| **Query Builder** | rule_type별 ES DSL 자동 생성, group_by 버킷 집계 |
| **룰 테스트 API** | Dry-run (샘플 로그), Backtest (과거 기간), 성능 측정 |
| **룰 버저닝** | 수정 시 버전업, 부모 룰 추적, 롤백 API |
| **중복 제거** | 동일 rule+group_key 1시간 내 재탐지 억제 |
| **통합 테스트** | Testcontainers로 ES 띄워 시나리오 테스트 (10개 룰) |
| **성능 벤치마크** | 1만 로그/룰 평가 < 1초, 메모리 < 500MB |

**승인 게이트**: 백테스트 시나리오 5개 통과, 성능 기준 충족

---

### Phase 4: 탐지 파이프라인 (2주차)

| 작업 | 완료 기준 |
|------|-----------|
| **스케줄러** | 룰별 주기 실행, 동시 실행 제어 (세마포어) |
| **배치 평가** | 윈도우 계산 -> ES 쿼리 -> CEL 평가 -> 저장 -> 알림 |
| **탐지 저장/조회 API** | 페이징, 필터 (심각도, 상태, 기간, 룰, IP) |
| **상태 머신** | open -> acknowledged -> investigating -> resolved/fp |
| **알림 디스패처** | 웹훅 (Slack/Teams), 대시보드 푸시, 재시도/데드레터 |
| **리포트 생성기** | 일/주/월 자동 생성, JSON/HTML 출력 |
| **장애 복구** | 스케줄러 재시작 시 미실행 윈도우 캐치업 |

**승인 게이트**: 24시간 무중단 운영 시나리오 통과

---

### Phase 5: 프론트엔드 MVP (2주차)

| 작업 | 완료 기준 |
|------|-----------|
| **프로젝트 초기화** | Vite + React 18 + TS + Shadcn/ui + Tailwind |
| **인증 플로우** | OIDC 리다이렉트, 토큰 갱신, 보호 라우트 |
| **대시보드** | 실시간 통계 카드, 트렌드 차트, Top 10, WS 구독 |
| **룰 관리** | 목록, 생성/수정 모달, CEL 에디터 |
| **탐지 현황** | 무한 스크롤 테이블, 상세 드로어, 로그 원문 뷰어 |
| **탐지 처리** | 상태 변경 (단일/일괄), 담당자 배정 |
| **리포트** | 목록, 생성 트리거, 미리보기, 다운로드 |
| **설정** | 데이터소스/알림 채널 등록/테스트 |
| **접근성/반응형** | WCAG AA, 모바일/데스크톱 대응 |
| **E2E 테스트** | Playwright 주요 플로우 10개 시나리오 통과 |

**승인 게이트**: 크로스 브라우저 테스트 통과

---

### Phase 6: 통합 테스트 & 문서화 (1주차)

| 작업 | 완료 기준 |
|------|-----------|
| **E2E 시나리오** | 로그 인제스트 -> 탐지 -> 알림 -> 대시보드 -> 리포트 전 과정 자동 검증 |
| **부하 테스트** | k6 동시 100유저, 룰 50개, 1시간 무장애 |
| **장애 주입** | ES 다운/지연, DB 락 시 graceful degradation 확인 |
| **운영 가이드** | 배포/롤백, 백업/복구, 트러블슈팅 런북 |
| **사용자 매뉴얼** | 분석가용, 관리자용 |
| **API 문서** | Swagger UI 배포, 예시 포함 |
| **보안 점검** | cargo-audit, 시크릿 스캔, OWASP Top 10 체크 |

**승인 게이트**: QA 팀 사인오프, 보안 팀 승인

---

### Phase 7: 파일럿 운영 (2주차)

| 작업 | 완료 기준 |
|------|-----------|
| **실 환경 연동** | 운영 ES 연결, 실 로그로 룰 평가 |
| **룰 튜닝** | False Positive < 5% 달성 |
| **성능 튜닝** | ES 쿼리 최적화, 평가 지연 < 500ms |
| **운영 피드백** | 분석가 인터뷰, UI/UX 개선 반영 |
| **문서 업데이트** | 실환경 설정값, 베스트 프랙티스 |
| **인수인계** | 운영팀 교육, 모니터링 연계 확인 |

**승인 게이트**: 운영팀 최종 승인, 프로덕션 전환

---

## 11. 리스크 및 대응

| 리스크 | 확률 | 영향도 | 대응 |
|--------|------|--------|------|
| CEL 성능 이슈 | 높음 | 높음 | 벤치마크, 복잡도 제한, 캐싱 |
| ES 쿼리 최적화 | 중간 | 높음 | 인덱스 사전 검증, 프로파일링 |
| 로그 필드 불일치 | 중간 | 중간 | 매핑표 검증, 필드 별칭 지원 |
| Keycloak 복잡도 | 낮음 | 높음 | 테스트 Realm, 문서화 |
| SQLite 동시성 | 낮음 | 중간 | WAL 모드, PG 마이그레이션 경로 |
| 프론트 실시간 성능 | 중간 | 중간 | 가상화, 디바운스, WS 재연결 |

---

## 12. 마이그레이션 경로 (SQLite -> PostgreSQL)

1. PostgreSQL 전용 타입 적용 (UUID, JSONB, TIMESTAMPTZ)
2. 인덱스 최적화 (Partial Index, BRIN for time-series)
3. 파티셔닝 (rule_executions by month)
4. 읽기 복제본 구성
5. 연결 풀 크기 조정 (PgBouncer)

**전환 트리거**: 일일 탐지 1만 건 초과, 동시 사용자 20명 초과

---

## 13. 향후 확장 로드맵

| 분기 | 목표 |
|------|------|
| Q4 2026 | 멀티 테넌시, RBAC 세분화 |
| Q1 2027 | ML 하이브리드 (Isolation Forest) |
| Q2 2027 | SOAR 연동 (자동 차단, 티켓 생성) |
| Q3 2027 | 위협 인텔리전스 (IOC 피드) |
| Q4 2027 | 클라우드 네이티브 (K8s Operator, GitOps) |

---

## 14. 변경 이력

| 버전 | 날짜 | 작성자 | 변경 내용 |
|------|------|--------|-----------|
| 1.0 | 2026-08-23 | - | 초안 작성 |
| 1.1 | 2026-08-23 | - | Phase 1 구현 완료 |

---

## 15. 구현 진행 상황

### Phase 1: 프로젝트 구조 설계 및 구현 (완료)

| 항목 | 상태 | 비고 |
|------|------|------|
| 스펙 문서 (SPEC.md) | ✅ 완료 | 885줄, 14 섹션 |
| 로그 필드 매핑 (LOG_FIELD_MAPPING.md) | ✅ 완료 | 391줄 |
| 초기 룰셋 (RULES.md) | ✅ 완료 | 10개 룰 |
| Rust 백엔드 워크스페이스 | ✅ 완료 | 4개 크레이트 |
| SQLite DB 마이그레이션 | ✅ 완료 | 7개 테이블 |
| React 프론트엔드 | ✅ 완료 | 7개 페이지 |
| API 핸들러 | ✅ 완료 | 5개 엔드포인트 |
| 프론트엔드-백엔드 연동 | ✅ 완료 | Vite 프록시 |
| Docker Compose | ✅ 완료 | 백엔드+프론트+ES |

### 테스트 결과

| 테스트 | 결과 |
|--------|------|
| `cargo check` | ✅ 통과 |
| `cargo build --release` | ✅ 통과 |
| `npm run build` | ✅ 통과 |
| API 엔드포인트 테스트 | ✅ 5/5 통과 |
| 프론트엔드-백엔드 연동 | ✅ 확인 |

### Phase 2: 룰 엔진 구현 (완료)

| 항목 | 상태 | 비고 |
|------|------|------|
| CEL 룰 엔진 | ✅ 완료 | cel-interpreter 기반 |
| 룰 실행 로직 | ✅ 완료 | RuleEngine (load, execute, save) |
| ES 클라이언트 확장 | ✅ 완료 | index, bulk, create, exists |
| Engine API | ✅ 완료 | POST /api/v1/engine/run |
| ES 타임아웃 처리 | ✅ 완료 | 5초 타임아웃, graceful handling |
| Docker Compose 전체 배포 | 🔲 예정 | Docker 미설치 |
| Keycloak OIDC 연동 | 🔲 예정 | |

### Phase 3: 룰 엔진 & ES 연동 (완료)

| 항목 | 상태 | 비고 |
|------|------|------|
| Rule CRUD API | ✅ 완료 | 생성/수정/삭제 + 자동 버전업 |
| 룰 테스트 API | ✅ 완료 | POST /rules/{id}/test |
| Detection 상태 변경 | ✅ 완료 | PATCH /detections/{id} |
| 프론트엔드 룰 생성/수정 UI | ✅ 완료 | RuleFormDialog |
| 프론트엔드 룰 테스트 UI | ✅ 완료 | RuleTestPanel |
| 프론트엔드 탐지 처리 UI | ✅ 완료 | Acknowledge/Resolve |

### Phase 4: 탐지 파이프라인 (완료)

| 항목 | 상태 | 비고 |
|------|------|------|
| 스케줄러 | ✅ 완료 | 60초 간격, 세마포어 기반 동시 실행 제어 |
| 알림 디스패처 | ✅ 완료 | 웹훅 기반 (Slack/Teams) |
| 배치 평가 | ✅ 완료 | 스케줄러에 통합 |
| 탐지 저장/조회 API | ✅ 완료 | Phase 3에서 구현 |
| 상태 머신 | ✅ 완료 | open -> acknowledged -> resolved/fp |
| 리포트 생성기 | ✅ 완료 | 일/주/월 보고서 자동 생성 |
| 탐지 필터링 | ✅ 완료 | 심각도/상태/기간별 필터 |

### Phase 5: 프론트엔드 MVP (완료)

| 항목 | 상태 | 비고 |
|------|------|------|
| 대시보드 차트 | ✅ 완료 | Recharts 기반 (타임라인, 심각도, Top 룰/IP) |
| 룰 관리 UI | ✅ 완료 | 목록, 생성/수정, 테스트 |
| 탐지 현황 | ✅ 완료 | 목록, 필터, 상세, 상태 변경 |
| 리포트 | ✅ 완료 | 목록, 일/주/월 생성 |
| 설정 | ✅ 완료 | 데이터소스/알림 채널 CRUD |
| 인증 | ✅ 완료 | Keycloak OIDC 연동 |

### API 엔드포인트 (전체)

| 메서드 | 경로 | 설명 | 상태 |
|--------|------|------|------|
| GET | `/health` | 헬스체크 | ✅ |
| GET | `/api/v1/rules` | 룰 목록 | ✅ |
| POST | `/api/v1/rules` | 룰 생성 | ✅ |
| GET | `/api/v1/rules/:id` | 룰 상세 | ✅ |
| PUT | `/api/v1/rules/:id` | 룰 수정 | ✅ |
| DELETE | `/api/v1/rules/:id` | 룰 삭제 | ✅ |
| POST | `/api/v1/rules/:id/test` | 룰 테스트 | ✅ |
| GET | `/api/v1/detections` | 탐지 목록 (필터 지원) | ✅ |
| GET | `/api/v1/detections/:id` | 탐지 상세 | ✅ |
| PATCH | `/api/v1/detections/:id` | 탐지 상태 변경 | ✅ |
| GET | `/api/v1/dashboard/stats` | 대시보드 통계 | ✅ |
| GET | `/api/v1/dashboard/timeline` | 24시간 타임라인 | ✅ |
| GET | `/api/v1/dashboard/top-rules` | 상위 룰 | ✅ |
| GET | `/api/v1/dashboard/top-ips` | 상위 IP | ✅ |
| POST | `/api/v1/engine/run` | 전체 룰 실행 | ✅ |
| POST | `/api/v1/engine/run/:id` | 단일 룰 실행 | ✅ |
| GET | `/api/v1/reports` | 리포트 목록 | ✅ |
| POST | `/api/v1/reports` | 리포트 생성 | ✅ |
| GET | `/api/v1/reports/:id` | 리포트 상세 | ✅ |
| GET | `/api/v1/data-sources` | 데이터소스 목록 | ✅ |
| POST | `/api/v1/data-sources` | 데이터소스 생성 | ✅ |
| DELETE | `/api/v1/data-sources/:id` | 데이터소스 삭제 | ✅ |
| POST | `/api/v1/data-sources/:id/test` | 데이터소스 테스트 | ✅ |
| GET | `/api/v1/notifications/channels` | 알림 채널 목록 | ✅ |
| POST | `/api/v1/notifications/channels` | 알림 채널 생성 | ✅ |
| DELETE | `/api/v1/notifications/channels/:id` | 알림 채널 삭제 | ✅ |
| POST | `/api/v1/notifications/channels/:id/test` | 알림 채널 테스트 | ✅ |
| GET | `/api/v1/auth/me` | 현재 사용자 | ✅ |
| GET | `/api/v1/auth/oidc/login` | OIDC 로그인 | ✅ |
| POST | `/api/v1/auth/oidc/callback` | OIDC 콜백 | ✅ |
