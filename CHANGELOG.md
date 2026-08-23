# CHANGELOG

AADS 프로젝트 변경 이력

## [0.1.0] - 2026-08-23

### Added

#### 프로젝트 구조
- SPEC.md: 시스템 스펙 문서 (14 섹션, 885줄)
- LOG_FIELD_MAPPING.md: DAOU-PF-OP-002 로그 필드 매핑 (391줄)
- RULES.md: 초기 10개 룰 정의 (CEL 표현식 + ES 쿼리 + 테스트 케이스)
- README.md: 프로젝트 가이드 및 API 문서
- CHANGELOG.md: 변경 이력

#### 백엔드 (Rust)
- 워크스페이스 구조: 4개 크레이트 (core, api, es, db)
- 설정 관리: Figment 기반 TOML 설정 (AppConfig, ServerConfig, DatabaseConfig, ElasticsearchConfig, OidcConfig)
- 데이터 모델: Rule, Detection, Report, DashboardStats (sqlx::FromRow)
- 상태 관리: AppState + ElasticSearchClientTrait
- 에러 처리: AppError (IntoResponse 구현)
- API 핸들러:
  - `GET /health` - 헬스체크
  - `GET /api/v1/rules` - 룰 목록 조회
  - `GET /api/v1/rules/{id}` - 룰 상세 조회
  - `GET /api/v1/detections` - 탐지 목록 조회
  - `GET /api/v1/dashboard/stats` - 대시보드 통계
- DB 레이어: SQLite 연결 + 수동 마이그레이션
- 마이그레이션:
  - 001_initial_schema.sql: 7개 테이블 (rules, rule_executions, rule_tests, reports, data_sources, notification_channels, users)
  - 002_seed_rules.sql: 10개 초기 룰 INSERT
- CORS 설정: `allow_origin(Any)` (개발 환경)
- Dockerfile: 멀티스테이지 빌드

#### 프론트엔드 (React)
- 설정: Vite + React 18 + TypeScript + Tailwind CSS + Shadcn/ui
- 라우팅: React Router v6 (/, /rules, /rules/:id, /detections, /detections/:id, /reports, /settings)
- API 클라이언트: Axios + 인증 인터셉터
- 타입 정의: TypeScript 인터페이스 (Rule, Detection, DashboardStats, etc.)
- 레이아웃: 사이드바 + 헤더
- 페이지:
  - DashboardPage: 통계 카드 (총 탐지, 열린 탐지, 활성 룰, 치명적 이슈)
  - RulesPage: 룰 목록 (심각도별 색상, 페이지네이션)
  - RuleDetailPage: 룰 상세 (정보 카드, 조건 표시)
  - DetectionsPage: 탐지 목록
- Vite 프록시: `/api/*` → `localhost:8080`
- Dockerfile: Node 빌드 + Nginx 서빙
- nginx.conf: 리버스 프록시 설정

#### 설정
- docker-compose.yml: 백엔드 + 프론트엔드 + ElasticSearch
- package.json: 개발 스크립트 (concurrently)
- backend/config.toml: 앱 설정

### 테스트 결과
- `cargo check`: ✅ 통과
- `cargo build --release`: ✅ 통과
- `npm run build`: ✅ 통과 (365KB JS, 45KB CSS)
- API 엔드포인트 테스트: ✅ 모두 통과
- 프론트엔드-백엔드 연동: ✅ 확인

### 초기 룰셋 (10개)

| ID | 이름 | 타입 | 심각도 |
|----|------|------|--------|
| rule-001 | Brute Force - Login | threshold | high |
| rule-002 | SQL Injection Attempt | pattern | critical |
| rule-003 | XSS Attempt | pattern | high |
| rule-004 | Path Traversal | pattern | high |
| rule-005 | Web Shell Access | pattern | critical |
| rule-006 | Privilege Escalation Attempt | composite | critical |
| rule-007 | Off-Hours Admin Access | composite | medium |
| rule-008 | Bot/Scanner Traffic | threshold | low |
| rule-009 | Data Exfiltration | threshold | high |
| rule-010 | Port Scan / Enumeration | threshold | medium |

## [0.2.0] - 2026-08-23

### Added

#### 백엔드 (Rust) - 룰 엔진
- engine 크레이트: aads-engine (CEL 룰 엔진)
- `crates/engine/src/cel.rs`: CEL 표현식 컴파일 및 평가 (CelEvaluator)
- `crates/engine/src/engine.rs`: 룰 실행 엔진 (RuleEngine)
  - `load_rules()`: DB에서 활성 룰 로드 및 CEL 컴파일
  - `execute_rule()`: 단일 룰 실행
  - `fetch_logs_from_es()`: ElasticSearch에서 로그 수집 (5초 타임아웃)
  - `save_detection()`: 탐지 결과 저장
  - `run_all_rules()`: 전체 룰 일괄 실행
- `crates/engine/src/types.rs`: 타입 정의 (LogEntry, DetectionResult, RuleContext)

#### API 엔드포인트
- `POST /api/v1/engine/run` - 전체 룰 실행
- `POST /api/v1/engine/run/{rule_id}` - 단일 룰 실행

#### ES 클라이언트 확장
- `index_document()`: 단일 문서 인덱싱
- `bulk_index()`: 벌크 인덱싱
- `create_index()`: 인덱스 생성
- `index_exists()`: 인덱스 존재 확인

#### 에러 처리
- `AppError::RuleEngine`: 룰 엔진 에러 variant 추가

### Changed
- `ElasticSearchClientTrait`: 4개 메서드 추가 (index_document, bulk_index, create_index, index_exists)
- 룰 엔진: ES 연결 실패 시 graceful handling (빈 결과 반환)
- ES 검색: 5초 타임아웃 적용

### 테스트 결과
- `cargo check`: ✅ 통과 (0 errors, 1 warning)
- `cargo build --release`: ✅ 통과
- Engine API 테스트: ✅ 10개 룰 일괄 실행 성공

## [0.3.0] - 2026-08-23

### Added

#### 백엔드 - Phase 3
- **Rule CRUD API**:
  - `POST /api/v1/rules` - 룰 생성
  - `PUT /api/v1/rules/{id}` - 룰 수정 (자동 버전업)
  - `DELETE /api/v1/rules/{id}` - 룰 소프트 삭제 (enabled=false)
- **룰 테스트 API**: `POST /api/v1/rules/{id}/test` - 드라이런 테스트 실행
- **탐지 상태 변경 API**: `PATCH /api/v1/detections/{id}` - 상태 업데이트 (acknowledged, resolved, false_positive)
- **요청 모델**: `CreateRuleRequest`, `UpdateRuleRequest`, `UpdateDetectionRequest`, `TestRuleRequest`

#### 백엔드 - Phase 4
- **스케줄러**: 60초 간격 전체 룰 일괄 실행, 세마포어 기반 동시 실행 제어 (최대 3개)
- **알림 디스패처**: 웹훅 기반 알림 발송 (Slack/Teams 호환)
- **리포트 생성기**: `ReportGenerator` - 일/주/월 보고서 자동 생성, JSON 출력
- **리포트 API**: `POST /api/v1/reports` - 리포트 생성, `GET /api/v1/reports` - 목록 조회

#### 백엔드 - Phase 5
- **대시보드 확장**: 타임라인, Top 룰, Top IP API 추가
  - `GET /api/v1/dashboard/timeline` - 24시간 타임라인
  - `GET /api/v1/dashboard/top-rules` - 상위 10개 룰
  - `GET /api/v1/dashboard/top-ips` - 상위 10개 IP
- **설정 API**: 데이터소스/알림 채널 CRUD
  - `GET/POST /api/v1/data-sources` - 데이터소스 목록/생성
  - `POST /api/v1/data-sources/{id}/test` - 연결 테스트
  - `GET/POST /api/v1/notifications/channels` - 알림 채널 목록/생성
  - `POST /api/v1/notifications/channels/{id}/test` - 테스트 발송
- **인증 모듈**: Keycloak OIDC 연동
  - `GET /api/v1/auth/me` - 현재 사용자 정보
  - `GET /api/v1/auth/oidc/login` - OIDC 로그인 URL
  - `POST /api/v1/auth/oidc/callback` - OIDC 콜백 처리

#### 프론트엔드 - Phase 3
- **룰 생성/수정 다이얼로그**: `RuleFormDialog` - 이름, 심각도, 타입, CEL 조건, 윈도우 설정
- **룰 테스트 패널**: `RuleTestPanel` - 원클릭 드라이런, 매칭 결과/실행 시간 표시
- **룰 상세 페이지**: 수정/삭제 기능, 테스트 패널 통합
- **룰 목록 페이지**: 페이지네이션 연동, 룰 생성 버튼
- **탐지 상세 페이지**: Acknowledge/Resolve 상태 변경 버튼

#### 프론트엔드 - Phase 4
- **대시보드 차트**: Recharts 기반 시각화
  - `TimelineChart` - 24시간 탐지 타임라인
  - `SeverityChart` - 심각도별 파이 차트
  - `TopRulesChart` - 상위 룰 가로 막대 차트
  - `TopIpsChart` - 상위 IP 가로 막대 차트
- **탐지 필터링**: 심각도/상태별 필터 UI
- **리포트 페이지**: 리포트 목록, 일/주/월 생성 버튼
- **설정 페이지**: 데이터소스/알림 채널 CRUD UI

### Changed
- `main.rs`: 스케줄러 백그라운드 태스크 시작, 신규 라우트 추가
- `engine` 크레이트: `scheduler`, `report` 모듈 추가, `reqwest` 의존성 추가
- `api` 크레이트: `auth`, `reports`, `settings` 핸들러 추가, `reqwest` 의존성 추가
- `models.rs`: 요청 구조체 추가 (CreateRuleRequest, UpdateRuleRequest, UpdateDetectionRequest, TestRuleRequest)
- `config.rs`: OidcConfig에 realm 필드 추가
- `docker-compose.yml`: OIDC redirect_url 환경 변수 추가

### 테스트 결과
- `cargo check`: ✅ 통과 (0 errors, 1 warning - upstream proc-macro-error2)
- `npm run build`: ✅ 통과 (776KB JS, 47KB CSS)
