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

## [0.3.0] - 예정

### planned
- Docker Compose 전체 배포 테스트
- Keycloak OIDC 연동
- 프로덕션 배포 가이드
- 프론트엔드 룰 엔진 연동 UI
