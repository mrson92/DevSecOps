# AADS - Abnormal Access Detection System

로그 표준화 기반 Access/Application Log에서 이상 접근을 탐지·분석·리포팅하는 시스템

## 아키텍처

```
+------------------+     +------------------+     +------------------+
|    Frontend      |     |     Backend      |     | ElasticSearch    |
|  React + Vite    | <-> |   Rust + Axum    | <-> |   (Log Store)    |
|  Shadcn/ui       |     |   SQLx + CEL     |     |                  |
+------------------+     +------------------+     +------------------+
                                |
                         +------+------+
                         |   SQLite    |
                         | (Rule Store)|
                         +-------------+
```

## 기술 스택

| 영역 | 기술 |
|------|------|
| **Frontend** | React 18, Vite, TypeScript, Tailwind CSS, Shadcn/ui |
| **Backend** | Rust, Axum, Tokio, SQLx, Elasticsearch-rs |
| **Rule Engine** | 네이티브 Rust 평가기 (regex 기반, CEL 대체) |
| **Database** | SQLite (MVP) → PostgreSQL (확장) |
| **Log Store** | ElasticSearch 8.x |
| **Auth** | Keycloak (OIDC) |
| **배포** | Podman / Docker Compose |

## 빠른 시작

### 개발 환경

```bash
# 1. 디펜던시 설치
npm install

# 2. 백엔드 실행 (터미널 1)
cd backend && cargo run

# 3. 프론트엔드 실행 (터미널 2)
cd frontend && npm run dev

# 또는 동시 실행
npm run dev
```

### Docker / Podman Compose

```bash
# 빌드 및 실행
docker compose up -d
# 또는 Podman 사용 시
podman compose up -d

# 로그 확인
docker compose logs -f
# 또는 Podman 사용 시
podman compose logs -f

# 중지 (볼륨 포함 삭제)
docker compose down -v
# 또는 Podman 사용 시
podman compose down -v
```

## 프로젝트 구조

```
DevSecOPS/
├── SPEC.md                    # 시스템 스펙 문서
├── LOG_FIELD_MAPPING.md       # 로그 필드 매핑표
├── docker-compose.yml         # Docker/Podman Compose 설정
├── package.json               # 루트 패키지 (dev 스크립트)
│
├── backend/                   # Rust 백엔드
│   ├── Cargo.toml             # 워크스페이스 설정
│   ├── config.toml            # 앱 설정
│   ├── Dockerfile             # 백엔드 Dockerfile (풀 빌드)
│   ├── Dockerfile.fast        # 빠른 배포용 Dockerfile (사전 빌드 바이너리)
│   ├── migrations/            # DB 마이그레이션
│   └── crates/
│       ├── core/              # 비즈니스 로직 (모델, 설정, 에러)
│       ├── api/               # HTTP 핸들러
│       ├── es/                # ElasticSearch 클라이언트
│       ├── db/                # 데이터베이스 레이어
│       └── engine/            # 룰 엔진 (네이티브 Rust 평가기)
│
└── frontend/                   # React 프론트엔드
    ├── package.json           # 의존성
    ├── vite.config.ts         # Vite 설정
    ├── Dockerfile             # 프론트 Dockerfile
    └── src/
        ├── app/               # 앱 라우팅
        ├── components/ui/     # Shadcn/ui 컴포넌트
        ├── features/          # 기능별 모듈 (dashboard, rules, detections, agents, chat, reports, settings)
        └── shared/            # 공통 모듈 (layout, types, api)
```

## API 엔드포인트

| 메서드 | 경로 | 설명 |
|--------|------|------|
| GET | `/health` | 헬스체크 |
| GET | `/api/v1/rules` | 룰 목록 |
| POST | `/api/v1/rules` | 룰 생성 |
| GET | `/api/v1/rules/:id` | 룰 상세 |
| PUT | `/api/v1/rules/:id` | 룰 수정 |
| DELETE | `/api/v1/rules/:id` | 룰 삭제 |
| POST | `/api/v1/rules/:id/test` | 룰 테스트 |
| GET | `/api/v1/mitre/tactics` | MITRE ATT&CK 전술 카탈로그 |
| GET | `/api/v1/mitre/techniques` | MITRE ATT&CK 기법 카탈로그 |
| GET | `/api/v1/detections` | 탐지 목록 |
| GET | `/api/v1/detections/:id` | 탐지 상세 |
| PATCH | `/api/v1/detections/:id` | 탐지 상태 업데이트 |
| GET | `/api/v1/dashboard/stats` | 대시보드 통계 |
| GET | `/api/v1/dashboard/timeline` | 타임라인 차트 |
| GET | `/api/v1/dashboard/top-rules` | 상위 룰 |
| GET | `/api/v1/dashboard/top-ips` | 상위 IP |
| POST | `/api/v1/logs/ingest` | 원시 로그 수집·정규화 (access/process) |
| POST | `/api/v1/engine/run` | 전체 룰 실행 |
| POST | `/api/v1/engine/run/:id` | 단일 룰 실행 |
| GET | `/api/v1/reports` | 리포트 목록 |
| POST | `/api/v1/reports` | 리포트 생성 (일/주/월) |
| GET | `/api/v1/reports/:id` | 리포트 상세 |
| DELETE | `/api/v1/reports/:id` | 리포트 삭제 |
| GET | `/api/v1/data-sources` | 데이터 소스 목록 |
| POST | `/api/v1/data-sources` | 데이터 소스 생성 |
| PUT | `/api/v1/data-sources/:id` | 데이터 소스 수정 |
| DELETE | `/api/v1/data-sources/:id` | 데이터 소스 삭제 |
| POST | `/api/v1/data-sources/:id/test` | 데이터 소스 연결 테스트 |
| GET | `/api/v1/notifications/channels` | 알림 채널 목록 |
| POST | `/api/v1/notifications/channels` | 알림 채널 생성 |
| DELETE | `/api/v1/notifications/channels/:id` | 알림 채널 삭제 |
| POST | `/api/v1/notifications/channels/:id/test` | 알림 채널 테스트 |
| GET | `/api/v1/agents` | 에이전트 목록 |
| POST | `/api/v1/agents` | 에이전트 생성 |
| GET | `/api/v1/agents/:id` | 에이전트 상세 |
| PUT | `/api/v1/agents/:id` | 에이전트 수정 |
| DELETE | `/api/v1/agents/:id` | 에이전트 삭제 |
| POST | `/api/v1/agents/:id/run` | 에이전트 즉시 실행 |
| GET | `/api/v1/agents/:id/runs` | 에이전트 실행 이력 |
| POST | `/api/v1/chat` | AI 채팅 |
| GET | `/api/v1/personas` | 페르소나 목록 |
| POST | `/api/v1/personas` | 페르소나 생성 |
| GET | `/api/v1/personas/:id` | 페르소나 상세 |
| PUT | `/api/v1/personas/:id` | 페르소나 수정 |
| DELETE | `/api/v1/personas/:id` | 페르소나 삭제 |
| GET | `/api/v1/settings/oidc` | OIDC 설정 |
| PUT | `/api/v1/settings/oidc` | OIDC 설정 수정 |
| POST | `/api/v1/settings/oidc/test` | OIDC 연결 테스트 |
| GET | `/api/v1/auth/me` | 현재 사용자 |
| GET | `/api/v1/auth/oidc/login` | OIDC 로그인 |
| POST | `/api/v1/auth/oidc/callback` | OIDC 콜백 |

## 스크립트

| 명령 | 설명 |
|------|------|
| `npm run dev` | 프론트+백엔드 동시 개발 서버 |
| `npm run build` | 전체 빌드 |
| `npm run test` | 전체 테스트 |
| `npm run lint` | 전체 린트 |
| `npm run docker:up` | Docker Compose 실행 |
| `npm run docker:down` | Docker Compose 중지 |

## 설정

환경 변수는 `backend/config.toml` 또는 환경 변수로 설정 가능:

| 변수 | 설명 | 기본값 |
|------|------|--------|
| `AADS_SERVER__ADDR` | 서버 주소 | `0.0.0.0:8080` |
| `AADS_DATABASE__URL` | DB URL | `sqlite:aads.db?mode=rwc` |
| `AADS_ELASTICSEARCH__URL` | ES URL | `http://localhost:9200` |
| `AADS_OIDC__ISSUER_URL` | OIDC Issuer | - |
| `AADS_OIDC__CLIENT_ID` | OIDC Client ID | - |
| `AADS_OIDC__JWT_SECRET` | JWT 시크릿 | - |

## 진행 상황

### 완료된 작업

| 단계 | 내용 | 상태 |
|------|------|------|
| Phase 1 | 프로젝트 구조 설계 및 구현 | ✅ 완료 |
| 1.1 | 스펙 문서 작성 (SPEC.md) | ✅ 완료 |
| 1.2 | 로그 필드 매핑 (LOG_FIELD_MAPPING.md) | ✅ 완료 |
| 1.3 | 초기 룰셋 정의 (10개 시드 룰) | ✅ 완료 |
| 1.4 | Rust 백엔드 워크스페이스 구성 | ✅ 완료 |
| 1.5 | SQLite DB 마이그레이션 | ✅ 완료 |
| 1.6 | React 프론트엔드 스캐폴딩 | ✅ 완료 |
| 1.7 | API 핸들러 구현 (전체 CRUD) | ✅ 완료 |
| 1.8 | 프론트엔드-백엔드 연동 | ✅ 완료 |
| Phase 2 | 룰 엔진 및 ELK 연동 | ✅ 완료 |
| 2.1 | ElasticSearch 로그 수집 | ✅ 완료 |
| 2.2 | 네이티브 Rust 룰 엔진 구현 | ✅ 완료 |
| 2.3 | 탐지 로직 구현 | ✅ 완료 |
| 2.4 | Podman/Docker Compose 배포 | ✅ 완료 |
| Phase 3 | UI 고도화 | ✅ 완료 |
| 3.1 | 대시보드 차트 (Recharts) | ✅ 완료 |
| 3.2 | 룰 관리 CRUD UI | ✅ 완료 |
| 3.3 | 탐지 상세/상태 관리 UI | ✅ 완료 |
| 3.4 | 리포트 UI | ✅ 완료 |
| 3.5 | 설정 페이지 (OIDC, 데이터소스, 알림) | ✅ 완료 |
| Phase 4 | 고급 기능 | ✅ 완료 |
| 4.1 | 리포트 생성 (일/주/월 + 과거 기간) 및 삭제 | ✅ 완료 |
| 4.2 | Sigma TAG/MITRE 메타데이터 (룰 태그, MITRE 카탈로그) | ✅ 완료 |
| 4.3 | Agent 실행 / 실행 이력 관리 | ✅ 완료 |
| 4.4 | AI 채팅 + 페르소나 | ✅ 완료 |
| Phase 5 | 위협 분석 파이프라인 (보안 위협 판단) | ✅ 완료 |
| 5.1 | 원시 로그 수집 (`POST /api/v1/logs/ingest`) | ✅ 완료 |
| 5.2 | security_stat 집계·적재 (Rule 검출 → ES) | ✅ 완료 |
| 5.3 | 1차 무감독 ML 이상탐지·점수화 | ✅ 완료 |
| 5.4 | 2차 LLM 위협 분석 (TP/FP 판정·심각도·조치) | ✅ 완료 |

### 룰 메타데이터 (Sigma TAG 참고)

룰에 MITRE ATT&CK 전술·기법과 자유 태그를 붙일 수 있습니다. MITRE 식별자(TAG)만 참조하고
Sigma 시그니처/본문은 복사하지 않는 방식을 사용합니다 (DRL 1.1 준수).

- `tags` : 자유 태그 (예: `attack.t1110`)
- `mitre_tactics` / `mitre_techniques` : MITRE ATT&CK 메타데이터
- RuleFormDialog에서 MITRE 카탈로그를 검색해 선택 가능
- RuleDetailPage에서 태그/전술/기법 배지로 표시
- 프론트엔드는 API가 JSON 문자열로 반환하는 배열 필드를 안전하게 파싱 (`parseStringArray`)

### 룰 엔진 지원 패턴

네이티브 Rust 평가기가 지원하는 조건 패턴:

| 패턴 | 예시 |
|------|------|
| 정규식 매칭 | `path.matches("(?i)/admin.*")` |
| 비교 연산 | `status_code >= 400`, `response_size >= 10485761` |
| 논리 연산 | `cond1 && cond2`, `cond1 \|\| cond2`, `!cond` |
| 필드 존재 | `has("user_id")` |
| 반복자 래퍼 | `exists(logs, log -> ...)`, `count(filter(logs, log -> ...)) >= N` |

### 보안 위협 판단 파이프라인

Rule 검출 결과를 다단계로 분석해 위협을 판단합니다.

```
[Raw Log]
   └─► (ingest) ──► [Rule Engine]
                        │
                        └─► [SecurityStat] (집계·요약, ES security_stat 인덱스)
                              │
                              ├─► ① 1차: 경량 무감독 ML 이상탐지 · 점수화
                              │      (Robust z-score 기반: matched_count, unique_ips,
                              │       unique_paths, unique_methods, status_4xx/5xx, error_rate)
                              │      → anomaly_score / threat_level (low~critical)
                              │
                              └─► ② 2차: LLM 위협 분석 (AgentRunner → persona)
                                     → TP/FP 판정 · 심각도 재평가 · MITRE 매핑 · 조치
```

- **1차 분석 (ML)**: 라벨 없는 무감독 이상탐지로 평소 패턴과 크게 다른 예외 이벤트를 선별해 LLM 리뷰 우선순위를 정한다.
- **2차 분석 (LLM)**: 1차 필터링된 `security_stat` + `threat_scores`를 페르소나(`persona-threat-analyst`)에 전달해 진탐/오탐·심각도·조치를 산출한다.

### 테스트 결과

| 테스트 | 결과 |
|--------|------|
| `cargo build --release` (백엔드 빌드) | ✅ 통과 |
| `npm run build` (프론트엔드 빌드) | ✅ 통과 |
| 전체 Docker/Podman 스택 배포 | ✅ 통과 |
| ES 클러스터 상태 (green) | ✅ 통과 |
| Backend 헬스체크 (`/health`) | ✅ 200 OK |
| Frontend 서빙 (Nginx) | ✅ 200 OK |
| API 프록시 (`/api/v1/*`) | ✅ 연동 확인 |
| 10개 룰 컴파일 및 스케줄러 실행 | ✅ 정상 동작 |
| `GET /api/v1/rules` | ✅ 10개 룰 조회 |
| `GET /api/v1/dashboard/stats` | ✅ 통계 조회 |

### 다음 단계

| 단계 | 내용 | 상태 |
|------|------|------|
| 5.5 | 지도학습 기반 오탐 필터링 (XGBoost/fp 학습) | 🔲 예정 |
| 5.6 | Keycloak OIDC 연동 | 🔲 예정 |
| 5.7 | 알림 채널 실제 전송 (Slack/이메일) | 🔲 예정 |
| 5.8 | 성능 테스트 및 최적화 | 🔲 예정 |

### 브라우저에서 확인

```
http://localhost:3000           → Dashboard
http://localhost:3000/rules     → Rules 목록
http://localhost:3000/rules/:id → Rule 상세 (태그/MITRE)
http://localhost:3000/detections → Detections 목록
http://localhost:3000/agents    → Agents 목록
http://localhost:3000/chat      → AI 채팅
http://localhost:3000/reports   → Reports 목록
http://localhost:3000/settings  → Settings (OIDC/데이터소스/알림)
```

## 라이선스

MIT
