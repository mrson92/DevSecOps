# UI Specification: Log Discovery & Analytics Dashboard

> **구현 현황(2026-09-07)**: 본 스펙은 Kibana/Grafana 스타일 로그 탐색 UI의 참조 문서이며,
> AADS 프론트엔드 **로그 브라우저(`/logs`)** 에 반영되었습니다.
> 아래 각 섹션의 `✅ 구현됨` / `⬜ 미구현` 표시는 현재 코드(`frontend/src/features/logs/`) 기준입니다.

## 1. Overview
Create a modern, dark/light-theme capable Log Discovery & Analytics dashboard UI inspired by observability tools like Grafana, Datadog, or VictoriaLogs. The layout consists of a left icon sidebar, an expandable fields sidebar, and a main content area containing search controls, a histogram chart, and a detailed data table.

**구현 현황**: ✅ 로그 브라우저(`/logs`)에 필드 사이드바 + 검색 컨트롤 + 히스토그램 + 상세 테이블 레이아웃 적용됨.
(왼쪽 아이콘 네비게이션 레일/전역 상단 헤더는 ⬜ 대시보드 레이아웃에 미반영 — 글로벌 픽셀 레이아웃 참고용)

---

## 2. Layout Structure

### A. Global Top Navigation Header
- **Left**:
  - Logo / Product Name (e.g., "LC")
  - Workspace / Team Dropdowns: `Demo Team` v / `WAF Security` v
  - Quick Info Badges: Datasource indicator (`VictoriaLogs`), Time Range (`Last 3h`), Timezone (`Local`), Limit (`100 rows`)
- **Right**:
  - Current Time Display (e.g., `14:26:17`)
  - Fullscreen toggle icon
  - Share icon
  - Settings icon

**구현 현황**: ⬜ 전역 헤더는 미구현(추후 레이아웃 개편 시 반영 예정).
데이터소스·시간범위·타임존·페이지크기 선택은 각각 헤더/툴바에서 부분 제공됨.

### B. Leftmost Navigation Rail (Icon Bar)
- Vertical bar containing icons for main app modules:
  - Search / Discovery (Active)
  - Dashboard
  - Alerts / Notifications
  - Metrics / Charts
  - Users / Teams
  - Integrations / API
  - Settings / Admin
  - User Profile Avatar (`DA` at the bottom)

**구현 현황**: ⬜ 아이콘 레일은 별도 구현 안 됨. AADS는 텍스트 기반 **Sidebar**(`shared/components/layout/Sidebar.tsx`)로 대체됨 (`Dashboard / Rules / Detections / Logs / Agents / Chat / Reports / Settings`).

### C. Left Sidebar (Fields Explorer)
- **Header**:
  - Title: "Fields"
  - Search input box: "Search fields..." with a refresh icon
- **Grouped Field Categories** (collapsible lists with icon, name, badge count, and action buttons):
  1. **CORE FIELDS (3)**
     - `_msg` (String) [click]
     - `L` (String) [click]
     - `_time` (DateTime)
  2. **CONTEXT FIELDS (2)**
     - `host` (String) [click]
     - `status` (String) [click]
  3. **FILTERABLE FIELDS (4)**
     - `duration_ms` (Number) (10)
     - `region` (String) [click]
     - `request_id` (String) [click]
     - `service_name` (String) [click]
  4. **SYSTEM FIELDS (2)**
     - `_stream` (String) [click]
     - `_stream_id` (String) [click]
- **Footer Legend**:
  - `+ Click value to add filter`
  - `- Click minus to exclude`

**구현 현황(코드 `FieldsPanel.tsx` 기준)**:
- ✅ Fields 사이드바(`w-80`, 접기/펼치기 버튼 포함) + "Search fields..." 필드 검색
- ⬜ 카테고리 그룹(CORE/CONTEXT/FILTERABLE/SYSTEM) 분류 — 현재는 전체 필드 목록 + 유형('F'=필터가능) 배지만 표시
- ✅ 필드 클릭 → 해당 필드의 **값 목록(집계)** 표시 (`/logs/fields/values`, size 10, hover 툴팁)
- ✅ 값 행 hover 시 **`+` / `−` 버튼** 표시:
  - `+`: 필드=값 **include** 필터 추가
  - `−`: 필드=값 **exclude(제외)** 필터 추가
- ✅ 하단 범례: `+ Click to add (include)` / `− Click to exclude`
- ✅ 집계 바(맥스 대비 상대 폭) + 값 카운트 표시

---

## 3. Main Content Area

### 1. Query & Search Bar Section
- **Mode Tabs**: `Search` (Selected), `LogsQL`, `AI Assistant`, `History`
- **Query Input Field**:
  - Monospace text editor styling
  - Current value: `lvl="ERROR"`
- **Right Action Group**:
  - Datasource selector: `VictoriaLogs`
  - `Collections` dropdown
  - `Live` streaming toggle button
  - Refresh Interval dropdown: `30s`
  - Primary Action Button: `Run` (Green background)

**구현 현황(코드 `LogsPage.tsx` 기준)**:
- ⬜ 모드 탭(Search/LogsQL/AI/History) — 현재 단일 검색창만 존재
- ✅ 모노스페이스 검색 입력(`path, URL, IP, UA...`) + Enter 실행
- ✅ 데이터소스 선택기 (`Auto(primary)` + 등록된 ClickHouse/ES 소스, ★ primary 표시)
- ✅ `Run`(Search) 버튼(primary) + `Reset` 버튼(outline)
- ⬜ Live 스트리밍 토글 / 새로고침 간격 선택 — 미구현

### 2. Histogram Section
- **Toolbar**:
  - Collapse/Expand chevron icon
  - Section Title: "Histogram"
  - Summary stats: `100 logs · 2ms`
  - Right controls: `Group By: No Grouping`, Share icon, View options (JSON/Raw/Chart)
- **Chart Component**:
  - Subtitle / Guide: "5m buckets · Hover to inspect · drag to select range · click bar to zoom"
  - Bar Chart: Vertical blue bars representing log frequency over time
  - X-Axis: Timestamps (e.g., `11:33`, `11:50`, `12:06`, `12:23`, etc.)
  - Y-Axis: Log counts (`0`, `2`, `4`, `6`, `8`, `10`)

**구현 현황(코드 `LogsPage.tsx` 기준)**:
- ✅ Timeline 히스토그램(Recharts Bar) — 현재 필터/시간범위와 동기화, 버킷 간격 자동(60s~1h)
- ✅ 서브가이드 표시: `{interval}s buckets · click/drag to select range`
- ✅ **클릭 브러시(시간 범위 선택)**: 1회 클릭으로 시작 버킷 선택(파란 강조) → 2회 클릭으로 종료 버킷 선택 시 해당 시간 범위로 `from/to` 재설정 후 자동 재검색. (첫 버킷 다시 클릭 시 해제)
- ✅ hover 툴팁(버킷 라벨 + 카운트)
- ⬜ Group By / JSON/Raw 뷰 토글 / 드래그 연속 선택 — 미구현 (클릭 브러시로 대체)

### 3. Log Results Table Section
- **Toolbar**:
  - Stats: `2ms · 100 rows`
  - Timezone Selector: `Local` | `UTC`
  - Page Size Dropdown: `50`
  - Pagination Controls: `<<` `<` `1/2` `>` `>>`
  - Column Configuration: `Columns (6)` button
  - Table Filter / Search Input: "Search..."
- **Data Table**:
  - **Headers**:
    - `_time` (Sortable)
    - `L` (Level)
    - `_msg` (Message)
    - `duration_ms`
    - `host`
    - `status`
  - **Row Elements & Styling**:
    - Expandable row arrow icons (`>`) on the far left
    - `_time`: Monospace timestamp (e.g., `2026-08-08T14:24:05.726+...`)
    - `L` (Log Level): Highlighting badge. Red background with white text for `ERROR`
    - `_msg`: Truncated log message text (e.g., `geo database looku...`, `upstream origin ti...`, `TLS handshake fail...`)
    - `duration_ms`: Numeric values right/left aligned (e.g., `21`, `47`, `72`, `299`)
    - `host`: Hostname string (e.g., `pop-nyc`, `pop-fra`, `edge-03`, `edge-02`)
    - `status`: Pill/Badge with HTTP status codes in red text/light red background (e.g., `504`, `502`, `403`, `500`, `429`)

**구현 현황(코드 `LogsPage.tsx` 기준)**:
- ✅ **타임존 선택기** (`Local` / `UTC`) — Local은 `ko-KR` 로케일, UTC는 ISO(공백 구분)로 표시
- ✅ **페이지 크기 선택기** (`25 / 50 / 100 / 200 / 500`)
- ✅ **컬럼 구성 버튼** (`Columns (N)`) — 드롭다운 체크박스로 표시 컬럼 추가/제거
  - 제공 컬럼: `timestamp, method, source, path, status_code, client_ip, user_id, user_agent, query, response_size, response_time`
- ✅ **확장 가능한 행(Expandable row)** — 첫 컬럼 `▸/▾` 토글, 행 클릭으로 열고 닫기
  - 펼침 시: 필드별 키-값 상세(2열 그리드) + `Raw JSON`(extra 필드) `<details>` 접기
- ✅ **배지 시각화**:
  - `status_code`: 5xx=red / 4xx=orange / 3xx=yellow / 그 외 green (pill)
  - `method`: blue pill
  - `source`: slate pill
- ✅ **필터 칩 바(Active Filters)** — 검색 바 아래 표시
  - 각 필터: `[NOT] 필드 = 값` 칩
  - `NOT` 텍스트 클릭 → include↔exclude 토글
  - `×` 클릭 → 해당 필터 제거
  - include=primary 색, exclude=destructive 색
  - 필드 값에서 `+`/`−`로 추가된 필터가 여기 축적됨
- ✅ Previous/Next 페이지네이션 + `datasource · total` 요약
- ⬜ 테이블 내 "Search..." 필터(결과 내 검색), `_time` 헤더 정렬 클릭, 시프트 페이지네이션(`<< >>`) — 미구현

---

## 4. UI/UX & Styling Guidelines
- **Color Palette**: Clean, crisp light mode (Slate/Gray border lines, white background `#FFFFFF`, soft gray panels `#F8FAFC`). Red error badges (`#EF4444` background / text), blue interactive accents (`#3B82F6` or `#2563EB`), green run button (`#10B981`).
- **Typography**: Monospace font for timestamps, log messages, query inputs, and status codes. Clean sans-serif font (Inter / Roboto) for UI navigation and field labels.
- **Interactivity**:
  - Table rows should highlight on hover.
  - Histogram bars should show tooltips on hover.
  - Sidebar fields should highlight with `+` / `-` filter buttons when hovered.

**구현 현황**:
- ✅ 행 hover 강조(`hover:bg-muted/50`), 선택 행 배경(`bg-muted/60`)
- ✅ 히스토그램 hover 툴팁 + 클릭 브러시
- ✅ 필드 값 hover 시 `+` / `−` 버튼 노출 (이미 활성 필터면 해당 버튼 색상 유지)
- ✅ 타임스탬프/쿼리/로그 메시지/status 코드 모노스페이스 폰트

---

## 5. 백엔드 연동 (2026-09-07 추가)

데이터 탐색 기능은 다음 API를 사용합니다:

| 엔드포인트 | 용도 |
|-----------|------|
| `GET /api/v1/logs` | 로그 검색 (페이징, 필터) |
| `GET /api/v1/logs/histogram` | 시간 버킷 집계 |
| `GET /api/v1/logs/fields` | 데이터소스 필드 스키마 목록 |
| `GET /api/v1/logs/fields/values` | 필드 값 집계(카운트) |

### 필터 파라미터

검색 파라미터에 **include/exclude(NOT)** 필터를 지원합니다.

| 파라미터 | 설명 |
|----------|------|
| `path`, `client_ip`, `method`, `source`, `status_code`, `user_id`, `user_agent` | include(포함) 필터 — ClickHouse `=`/`ILIKE`, ES `term`/`wildcard` |
| `exclude_path`, `exclude_client_ip`, `exclude_method`, `exclude_source`, `exclude_status_code` | exclude(제외) 필터 — ClickHouse `NOT`/`NOT ILIKE`, ES `must_not` |
| `exclude_status_code` | 상태 코드 제외(숫자만 허용) |

예시:
```bash
# POST 제외, /admin 제외, 404 제외
curl "http://localhost:8080/api/v1/logs?exclude_method=POST&exclude_path=/admin&exclude_status_code=404"
```

### 구현 노트
- 프론트는 필드 값의 `+`/`−` 클릭으로 `AppliedFilters[]`(칩)를 만들어 검색 쿼리에 반영.
- 히스토그램은 검색과 동일한 필터를 공유하므로, 필터 변경 시 차트와 테이블이 함께 갱신됨.
- 브러시(클릭 2회)로 시간 범위를 선택하면 `from`/`to`가 ISO로 재설정되어 자동 재검색됨.