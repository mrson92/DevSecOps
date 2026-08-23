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
| **Database** | SQLite (MVP) → PostgreSQL (확장) |
| **Log Store** | ElasticSearch 8.x |
| **Auth** | Keycloak (OIDC) |
| **배포** | Docker Compose |

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

### Docker Compose

```bash
# 빌드 및 실행
docker compose up -d

# 로그 확인
docker compose logs -f

# 중지
docker compose down
```

## 프로젝트 구조

```
DevSecOPS/
├── SPEC.md                    # 시스템 스펙 문서
├── LOG_FIELD_MAPPING.md       # 로그 필드 매핑표
├── docker-compose.yml         # Docker Compose 설정
├── package.json               # 루트 패키지 (dev 스크립트)
│
├── backend/                   # Rust 백엔드
│   ├── Cargo.toml             # 워크스페이스 설정
│   ├── config.toml            # 앱 설정
│   ├── Dockerfile             # 백엔드 Dockerfile
│   ├── migrations/            # DB 마이그레이션
│   └── crates/
│       ├── core/              # 비즈니스 로직 (모델, 설정, 에러)
│       ├── api/               # HTTP 핸들러
│       ├── es/                # ElasticSearch 클라이언트
│       └── db/                # 데이터베이스 레이어
│
└── frontend/                  # React 프론트엔드
    ├── package.json           # 의존성
    ├── vite.config.ts         # Vite 설정
    ├── Dockerfile             # 프론트 Dockerfile
    └── src/
        ├── app/               # 앱 라우팅
        ├── components/ui/     # Shadcn/ui 컴포넌트
        ├── features/          # 기능별 모듈 (dashboard, rules, detections)
        └── shared/            # 공통 모듈 (layout, types, api)
```

## API 엔드포인트

| 메서드 | 경로 | 설명 |
|--------|------|------|
| GET | `/health` | 헬스체크 |
| GET | `/api/v1/rules` | 룰 목록 |
| GET | `/api/v1/rules/:id` | 룰 상세 |
| GET | `/api/v1/detections` | 탐지 목록 |
| GET | `/api/v1/detections/:id` | 탐지 상세 |
| GET | `/api/v1/dashboard/stats` | 대시보드 통계 |

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
| 1.3 | 초기 룰셋 정의 (RULES.md) | ✅ 완료 |
| 1.4 | Rust 백엔드 워크스페이스 구성 | ✅ 완료 |
| 1.5 | SQLite DB 마이그레이션 | ✅ 완료 |
| 1.6 | React 프론트엔드 스캐폴딩 | ✅ 완료 |
| 1.7 | API 핸들러 구현 | ✅ 완료 |
| 1.8 | 프론트엔드-백엔드 연동 | ✅ 완료 |

### 테스트 결과

| 테스트 | 결과 |
|--------|------|
| `cargo check` (백엔드 컴파일) | ✅ 통과 |
| `cargo build --release` (백엔드 빌드) | ✅ 통과 |
| `npm run build` (프론트엔드 빌드) | ✅ 통과 |
| `GET /health` | ✅ 200 OK |
| `GET /api/v1/rules` | ✅ 10개 룰 조회 |
| `GET /api/v1/dashboard/stats` | ✅ 통계 조회 |
| `GET /api/v1/detections` | ✅ 빈 목록 조회 |
| Vite API 프록시 | ✅ 연동 확인 |

### 다음 단계 (Phase 2)

| 단계 | 내용 | 상태 |
|------|------|------|
| 2.1 | ElasticSearch 로그 수집 | 🔲 예정 |
| 2.2 | 룰 엔진 구현 (CEL) | 🔲 예정 |
| 2.3 | 탐지 로직 구현 | 🔲 예정 |
| 2.4 | Docker Compose 배포 | 🔲 예정 |
| 2.5 | Keycloak OIDC 연동 | 🔲 예정 |

### 브라우저에서 확인

```
http://localhost:3000           → Dashboard
http://localhost:3000/rules     → Rules 목록
http://localhost:3000/detections → Detections 목록
```

## 라이선스

MIT
