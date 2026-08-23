# 로그 표준화 필드 매핑표 (DAOU-PF-OP-002 → ElasticSearch)

## 1. 개요

| 항목 | 내용 |
|------|------|
| **기반 문서** | DAOU-PF-OP-002 로그표준화_v1.92_20250108 |
| **매핑 대상** | ElasticSearch 인덱스 템플릿 (SPEC.md 5.1) |
| **대상 로그** | 프로세스 로그 (Application Log), 웹 Access 로그 |
| **웹 서버** | Tomcat, Apache |

---

## 2. 프로세스 로그 (Application Log) 필드 매핑

### 2.1 로그 포맷 (표준)
```
%d{yyyy-MM-dd'T'HH:mm:ss.SSSZ} DO %X{session.id} %X{attr.email} [%thread] %-5level %logger{36}[%method:%line] - %msg%n
```

### 2.2 샘플 로그
```
2021-09-15T00:02:16.324+0900 DO 8e51aeb7-eb5b-49c7-a9a9-efbe1ac5e66b ********@daouoffice.com [http-nio-80-exec-278] WARN  c.d.g.a.h.dc.DocAgreementBoxSetter[lambda$getOrder$2:92] - no matching agreement group found in html.
```

### 2.3 필드 매핑 테이블

| # | 로그 필드 | 설명 | 필수 | 샘플 값 | ES 경로 | ES 타입 | 매핑 노트 |
|---|-----------|------|------|---------|---------|---------|-----------|
| 1 | Date | 처리 날짜/시간 (밀리초) | O | `2021-09-15T00:02:16.324+0900` | `@timestamp` | date | ISO 8601 또는 epoch_millis로 변환 필수 |
| 2 | Level | 로그 레벨/심각도 | O | `WARN` | `log.level` | keyword | ERROR/WARN/INFO/DEBUG/TRACE |
| 3 | 서비스 식별자 | 서비스 구분 코드 | O | `DO` | `app.service` | keyword | 2자리 대문자 코드 (DO, BP, EN 등) |
| 4 | 세션 식별자 | 처리 세션 ID | X | `8e51aeb7-eb5b-49c7-a9a9-efbe1ac5e66b` | `app.user.session_id` | keyword | UUID 형식 |
| 5 | 사용자 식별자 | 사용자 이메일/ID | X | `********@daouoffice.com` | `app.user.id` | keyword | 값 없으면 `-` 표기 |
| 6 | thread | 처리 스레드 식별자 | O | `http-nio-80-exec-278` | `app.instance` | keyword | Tomcat/Apache 스레드명 |
| 7 | logger | 로그 출력 클래스 | O | `c.d.g.a.h.dc.DocAgreementBoxSetter` | `log.logger` | keyword | fully-qualified class name |
| 8 | method | 메소드명 | X | `lambda$getOrder$2` | `log.logger` 에 포함 | - | `logger[method:line]` 형식으로 결합 |
| 9 | line | 라인 번호 | X | `92` | `log.logger` 에 포함 | - | `logger[method:line]` 형식으로 결합 |
| 10 | msg | 출력 메시지 | O | `no matching agreement group found in html.` | `log.message` | text | full-text 검색 가능 (korean analyzer) |

### 2.4 파싱 규칙 (Logback → ES)

```
원본: 2021-09-15T00:02:16.324+0900 DO 8e51aeb7-... ********@daouoffice.com [http-nio-80-exec-278] WARN  c.d.g.a.h.dc.DocAgreementBoxSetter[lambda$getOrder$2:92] - no matching...

파싱 결과:
  @timestamp      = 2021-09-15T00:02:16.324+09:00
  app.service     = "DO"
  app.user.session_id = "8e51aeb7-eb5b-49c7-a9a9-efbe1ac5e66b"
  app.user.id     = "********@daouoffice.com"
  app.instance    = "http-nio-80-exec-278"
  log.level       = "WARN"
  log.logger      = "c.d.g.a.h.dc.DocAgreementBoxSetter[lambda$getOrder$2:92]"
  log.message     = "no matching agreement group found in html."
  log.type        = "application"
```

---

## 3. 웹 Access 로그 (Tomcat) 필드 매핑

### 3.1 로그 포맷 (표준)
```
%{X-Forwarded-For}i DO %I %{email}r [%{yyyy-MM-dd'T'HH:mm:ss.SSSZ}t] %r %s %b "%{Referer}i" "%{User-Agent}i" "%{GO-Agent}i" %T
```

### 3.2 샘플 로그
```
112.220.20.130 DO http-nio-80-exec-337 *****@daouoffice.com  [15/Sep/2021:00:00:00 +0900] GET /api/ehr/timeline/info HTTP/1.1 200 539 "https://nsoft.daouoffice.com/app/home" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.63 Safari/537.36" "" 0.134
```

### 3.3 필드 매핑 테이블

| # | 로그 필드 | 설명 | 필수 | 샘플 값 | ES 경로 | ES 타입 | 매핑 노트 |
|---|-----------|------|------|---------|---------|---------|-----------|
| 1 | Client IP | 접속 IP (프록시/LB 포함) | O | `112.220.20.130` | `network.client.ip` | ip | X-Forwarded-For 우선, 없으면 Remote Addr |
| 2 | 서비스 식별자 | 서비스 구분 코드 | X | `DO` | `app.service` | keyword | 2자리 대문자 코드 |
| 3 | Thread/Process ID | 처리 쓰레드 식별자 | O | `http-nio-80-exec-337` | `app.instance` | keyword | 톰캣: %I, 아파치: http-%P-%p |
| 4 | User ID | 사용자 식별자 | O | `*****@daouoffice.com` | `app.user.id` | keyword | 값 없으면 `-` 표기 |
| 5 | Date/Time | 처리 시간 | O | `15/Sep/2021:00:00:00 +0900` | `@timestamp` | date | Apache 결합형 날짜 형식으로 변환 |
| 6 | Request | 요청 정보 (Method/URL/Proto) | O | `GET /api/ehr/timeline/info HTTP/1.1` | `http.request.method` / `http.request.path` / `http.request.version` | keyword | Request를 3개 필드로 분리 |
| 7 | Status Code | 서버 응답 코드 | O | `200` | `http.response.status_code` | short | HTTP 상태 코드 |
| 8 | Byte Sent | 응답 크기 (bytes) | O | `539` | `http.response.size` | long | 바이트 단위 |
| 9 | Referer | Referer URL | O | `https://nsoft.daouoffice.com/app/home` | `http.request.headers.referer` | keyword | 호출 원본 페이지 |
| 10 | User-Agent | 사용자 에이전트 | O | `Mozilla/5.0 ...` | `http.user_agent.original` | keyword | 브라우저/OS/디바이스 파싱 필요 |
| 11 | 추가 정보 | 커스텀 헤더 등 | X | `""` | `log.original` | text | 비정형 데이터 저장 |
| 12 | Processing Time | 처리 소요 시간 (초) | O | `0.134` | `http.response.latency_ms` | integer | 초 → 밀리초 변환 필수 (0.134 → 134) |

### 3.4 파싱 규칙 (Tomcat Access Log → ES)

```
원본: 112.220.20.130 DO http-nio-80-exec-337 *****@daouoffice.com [15/Sep/2021:00:00:00 +0900] GET /api/ehr/timeline/info HTTP/1.1 200 539 "https://..." "Mozilla/5.0..." "" 0.134

파싱 결과:
  @timestamp                 = 2021-09-15T00:00:00.000+09:00
  network.client.ip          = "112.220.20.130"
  app.service                = "DO"
  app.instance               = "http-nio-80-exec-337"
  app.user.id                = "*****@daouoffice.com"
  http.request.method        = "GET"
  http.request.path          = "/api/ehr/timeline/info"
  http.request.version       = "HTTP/1.1"
  http.response.status_code  = 200
  http.response.size         = 539
  http.user_agent.original   = "Mozilla/5.0 ..."
  http.response.latency_ms   = 134
  log.type                   = "access"
```

---

## 4. 웹 Access 로그 (Apache) 필드 매핑

### 4.1 로그 포맷 (표준)
```
%h BP http-%{pid}P-%{remote}p %u [%{%Y-%m-%d %H:%M:%S}t.%{msec_frac}t] "%r" %>s %b "%{Referer}i" "%{User-Agent}i" "" %T
```

### 4.2 샘플 로그
```
172.21.25.25 BP http-18382-57764 - [2025-01-08 15:52:52.805] "GET /main/notice/list HTTP/1.1" 200 1075 "https://dev.bizppurio.com:14119/" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Whale/4.29.282.14 Safari/537.36" "" 60
```

### 4.3 필드 매핑 테이블

| # | 로그 필드 | 설명 | 필수 | 샘플 값 | ES 경로 | ES 타입 | 매핑 노트 |
|---|-----------|------|------|---------|---------|---------|-----------|
| 1 | %h (Client IP) | 접속 IP | O | `172.21.25.25` | `network.client.ip` | ip | 직접 접속 IP |
| 2 | 서비스 식별자 | 서비스 구분 코드 | X | `BP` | `app.service` | keyword | 2자리 대문자 코드 |
| 3 | Thread/Process ID | 처리 프로세스-쓰레드 | O | `http-18382-57764` | `app.instance` | keyword | Apache PID-Connection |
| 4 | %u (User ID) | 사용자 식별자 | X | `-` | `app.user.id` | keyword | Apache 인증 미사용 시 `-` |
| 5 | Date/Time | 처리 시간 | O | `2025-01-08 15:52:52.805` | `@timestamp` | date | `yyyy-MM-dd HH:mm:ss.SSS` 형식 |
| 6 | %r (Request) | 요청 정보 | O | `GET /main/notice/list HTTP/1.1` | `http.request.method` / `http.request.path` / `http.request.version` | keyword | 3개 필드로 분리 |
| 7 | %>s (Status) | 응답 코드 | O | `200` | `http.response.status_code` | short | HTTP 상태 코드 |
| 8 | %b (Byte Sent) | 응답 크기 | O | `1075` | `http.response.size` | long | 바이트 단위 |
| 9 | Referer | Referer URL | O | `https://dev.bizppurio.com:14119/` | `http.request.headers.referer` | keyword | 호출 원본 페이지 |
| 10 | User-Agent | 사용자 에이전트 | O | `Mozilla/5.0 ...` | `http.user_agent.original` | keyword | 브라우저/OS 파싱 |
| 11 | 추가 정보 | 커스텀 데이터 | X | `""` | `log.original` | text | 비정형 데이터 |
| 12 | %T (Time) | 처리 소요 시간 (초) | O | `60` | `http.response.latency_ms` | integer | 초 → 밀리초 변환 (60 → 60000) |

---

## 5. 서비스 코드 매핑 테이블

| 코드 | 서비스명 | 영문명 |
|------|----------|--------|
| PP | 뿌리오 | ppurio |
| EN | 엔팩스 | enpax |
| HS | 반값문자 | half SMS |
| MZ | 알뜰장문 | mzone |
| SM | 문자매니아 | SMS mania |
| BM | 비즈메일러 | Biz Mailer |
| CP | 도넛북(쿠팝) | Donutbook(Kooup) |
| UC | 유니크로 | Unique |
| B3 | 배달365 | Delivery365 |
| TP | 텔패스 | Telpass |
| CM | 콜믹스 | Callmix |
| UF | 유핏 | Ufit |
| BP | 비즈뿌리오 | Biz Ppurio |
| SB | 사방넷 | Sabangnet |
| DO | 다우오피스 | DaouOffice |
| DA | 경리회계 | Accounting |
| DE | 경리회계 경영지원 | Accounting Support |
| PL | 배달대행 플레이 | Delivery Play |
| DC | CMS | CMS |
| IC | 인터비즈CMS | Interbiz CMS |
| CA | 쿠팝ASP | Kooup ASP |
| BC | 비즈쿠팝 | Biz Kooup |
| BR | 브랜드사 정산 시스템 | Brand Settlement |
| BS | 영업관리시스템 | BizSales |
| NB | 번호자원관리시스템 | NumBall |
| IV | 080수신거부시나리오 | IVR |
| NP | JavaASP | JavaASP |
| KA | 교촌치킨 ASP | Kyochon ASP |
| SL | 셀러 | Seller |

---

## 6. 로그 레벨 매핑

### 6.1 표준 레벨 정의

| 중요도 | 종류 | 상황 | ES 매핑 |
|--------|------|------|---------|
| 5 | ERROR | 비정상, 자체 복구 불가, 즉시 조치 필요 | `log.level: ERROR` |
| 4 | WARN | 비정상, 자체 복구 가능, 빠른 수정 필요 | `log.level: WARN` |
| 3 | INFO | 정상, 개발/운영에 도움 되는 정보 | `log.level: INFO` |
| 2 | DEBUG | 디버깅용 정보 | `log.level: DEBUG` |
| 1 | TRACE | 디버깅용 상세 정보 | `log.level: TRACE` |

### 6.2 레벨별 ES 활용

| 레벨 | 이상탐지 활용 | ES 인덱싱 전략 |
|------|---------------|----------------|
| ERROR | 시스템 장애 탐지, 서비스 중단 알림 | 즉시 인덱싱, 장기 보관 (30일+) |
| WARN | 비정상 패턴 탐지 (로그인 실패 등) | 즉시 인덱싱, 탐지 대상 |
| INFO | 접근 패턴 분석, 사용량 통계 | 10초 지연 인덱싱, 30일 보관 |
| DEBUG | 개발 분석용 | 선택적 인덱싱, 7일 보관 |
| TRACE | 상세 디버깅 | 인덱싱 없음 (파일 보관만) |

---

## 7. 값 변환 규칙

### 7.1 시간 포맷 변환

| 원본 포맷 | 변환 규칙 | ES 저장 형식 |
|-----------|-----------|--------------|
| `yyyy-MM-dd'T'HH:mm:ss.SSSZ` | 직접 매핑 | ISO 8601 |
| `yyyy-MM-dd HH:mm:ss.SSS` | 직접 매핑 | ISO 8601 |
| `dd/MMM/yyyy:HH:mm:ss Z` (Apache 결합형) |月份 영문 → 숫자 변환 | ISO 8601 |
| `%T` (초 단위, Apache) | `× 1000` → 밀리초 | 정수 (latency_ms) |
| `%T` (초 단위, Tomcat) | `× 1000` → 밀리초 | 정수 (latency_ms) |

**月份 변환 테이블 (Apache 결합형)**:

| 원본 | 숫자 |
|------|------|
| Jan | 01 |
| Feb | 02 |
| Mar | 03 |
| Apr | 04 |
| May | 05 |
| Jun | 06 |
| Jul | 07 |
| Aug | 08 |
| Sep | 09 |
| Oct | 10 |
| Nov | 11 |
| Dec | 12 |

### 7.2 Request 파싱 규칙

```
원본: "GET /api/ehr/timeline/info HTTP/1.1"

파싱:
  http.request.method  = "GET"     (공백 기준 1번 토큰)
  http.request.path    = "/api/ehr/timeline/info"  (공백 기준 2번 토큰)
  http.request.version = "HTTP/1.1"  (공백 기준 3번 토큰)
```

### 7.3 User-Agent 파싱

| 필드 | 파싱 방법 | ES 저장 |
|------|-----------|---------|
| Browser | UA 문자열에서 브라우저 감지 | `http.user_agent.browser` |
| OS | UA 문자열에서 OS 감지 | `http.user_agent.os` |
| Device | UA 문자열에서 디바이스 감지 | `http.user_agent.device` |
| Is Bot | 봇/크롤러 패턴 매칭 | `http.user_agent.is_bot` |

### 7.4 IP 주소 처리

```
원본: X-Forwarded-For = "client_ip, proxy1, proxy2"

규칙:
  1. X-Forwarded-For가 있으면: 첫 번째 IP 사용 (client_ip)
  2. X-Forwarded-For가 없으면: Remote Addr 사용
  3. IPv6 → IPv4 매핑 필요 시 변환
  4. 프라이빗 IP 대역 (10.x, 172.16-31.x, 192.168.x) 태깅
```

---

## 8. 필수 파싱 필드 (Fluent Bit / Vector 설정 참조)

### 8.1 Fluent Bit Parser 설정 예시

```ini
[PARSER]
    Name         daou_access
    Format       regex
    Regex        ^(?<client_ip>\S+) (?<service>\S+) (?<thread>\S+) (?<user>\S+) \[(?<time>[^\]]+)\] (?<method>\S+) (?<path>\S+) (?<protocol>\S+) (?<status>\d+) (?<bytes>\d+) "(?<referer>[^"]*)" "(?<user_agent>[^"]*)" "" (?<latency>\S+)
    Time_Key     time
    Time_Format  %d/%b/%Y:%H:%M:%S %z

[PARSER]
    Name         daou_process
    Format       regex
    Regex        ^(?<time>\S+ \S+ \S+) (?<service>\S+) (?<session_id>\S+) (?<user>\S+) \[(?<thread>\S+)\] (?<level>\S+)\s+(?<logger>\S+)\[(?<method>\S+):(?<line>\d+)\] - (?<message>.*)$
    Time_Key     time
    Time_Format  %Y-%m-%dT%H:%M:%S.%L%z
```

### 8.2 Vector Transform 설정 예시 (TOML)

```toml
[transforms.parse_access]
type = "remap"
inputs = ["raw_access"]
source = '''
  .timestamp = parse_timestamp!(.timestamp, "%d/%b/%Y:%H:%M:%S %z")
  .http.request.method = split!(.request, " ")[0]
  .http.request.path = split!(.request, " ")[1]
  .http.request.version = split!(.request, " ")[2]
  .http.response.status_code = to_int!(.status)
  .http.response.size = to_int!(.bytes)
  .http.response.latency_ms = to_int!(.latency) * 1000
  .app.service = .service_code
  .network.client.ip = .client_ip
  .log.type = "access"
'''
```

---

## 9. 엔리치먼트 규칙 (파싱 후 추가 정보)

### 9.1 GeoIP 엔리치먼트

| 입력 필드 | 엔리치먼트 필드 | 방법 |
|-----------|----------------|------|
| `network.client.ip` | `network.client.geo.country` | GeoIP DB (MaxMind GeoLite2) |
| `network.client.ip` | `network.client.geo.city` | GeoIP DB |
| `network.client.ip` | `network.client.geo.location` | GeoIP DB (geo_point) |
| `network.client.ip` | `network.client.geo.asn` | ASN DB |

### 9.2 위험 점수 엔리치먼트

| 조건 | 위험 점수 추가 |
|------|----------------|
| 외부 IP (프라이빗 대역 아님) | +10 |
| 봇/크롤러 User-Agent | +20 |
| Known 공격 IP (Threat Intel) | +50 |
| 비업무 시간 (00:06~08:59) 접근 | +15 |
| 관리자 경로 접근 (/admin, /manage) | +25 |

### 9.3 MITRE ATT&CK 자동 태깅

| 패턴 | 기법 | 전술 |
|------|------|------|
| 로그인 실패 다수 | T1110 | Credential Access |
| SQL 키워드 감지 | T1190 | Initial Access |
| XSS 패턴 감지 | T1189 | Initial Access |
| 경로 순회 (../../) | T1083 | Discovery |
| 웹쉘 파일 접근 | T1505.003 | Persistence |
| 관리자 API 접근 | T1078 | Privilege Escalation |
| 대용량 응답 | T1048 | Exfiltration |
| 다수 경로 탐색 | T1046 | Discovery |

---

## 10. 샘플 데이터 (테스트용)

### 10.1 프로세스 로그 샘플 (10건)

```
2026-08-23T09:15:30.123+0900 DO 8e51aeb7-eb5b-49c7-a9a9-efbe1ac5e66b user@daouoffice.com [http-nio-80-exec-278] INFO  c.d.o.a.service.UserService[login:142] - User login successful
2026-08-23T09:15:31.456+0900 DO a1b2c3d4-e5f6-7890-abcd-ef1234567890 admin@daouoffice.com [http-nio-80-exec-12] WARN  c.d.o.a.filter.SecurityFilter[doFilter:89] - Suspicious request detected from 203.0.113.50
2026-08-23T09:15:32.789+0900 BP f1e2d3c4-b5a6-0987-dcfe-edcba9876543 - [batch-processor-1] ERROR c.d.b.j.JobExecutor[run:234] - Database connection timeout
2026-08-23T02:15:33.012+0900 DO 11111111-2222-3333-4444-555555555555 admin@daouoffice.com [http-nio-80-exec-99] INFO  c.d.o.a.controller.AdminController[accessLog:67] - Admin panel accessed
2026-08-23T09:15:34.345+0900 DO 22222222-3333-4444-5555-666666666666 user1@daouoffice.com [http-nio-80-exec-45] WARN  c.d.o.a.service.AuthService[verify:198] - Token expired, refresh required
2026-08-23T09:15:35.678+0900 EN 33333333-4444-5555-6666-777777777777 user2@enpax.co.kr [http-nio-80-exec-67] INFO  c.d.e.a.service.FaxService[send:312] - Fax transmission initiated
2026-08-23T09:15:36.901+0900 BP 44444444-5555-6666-7777-888888888888 user3@bizppurio.com [http-nio-80-exec-23] ERROR c.d.b.a.controller.ApiController[handle:156] - Rate limit exceeded
2026-08-23T09:15:37.234+0900 DO 55555555-6666-7777-8888-999999999999 - [health-check] INFO  c.d.o.a.health.HealthCheck[check:45] - All services healthy
2026-08-23T09:15:38.567+0900 DO 66666666-7777-8888-9999-aaaaaaaaaaaa user4@daouoffice.com [http-nio-80-exec-88] WARN  c.d.o.a.service.FileService[upload:234] - File size exceeds limit: 50MB
2026-08-23T09:15:39.890+0900 SL 77777777-8888-9999-aaaa-bbbbbbbbbbbb seller@seller.co.kr [http-nio-80-exec-55] INFO  c.d.s.a.service.OrderService[create:178] - New order created: ORD-2026-001
```

### 10.2 Access 로그 샘플 - Tomcat (10건)

```
112.220.20.130 DO http-nio-80-exec-337 user1@daouoffice.com  [2026-08-23T09:15:30.123+0900] GET /api/ehr/timeline/info HTTP/1.1 200 539 "https://nsoft.daouoffice.com/app/home" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36" "" 0.134
10.0.1.55 DO http-nio-80-exec-12 admin@daouoffice.com  [2026-08-23T09:15:31.456+0900] POST /api/admin/users/create HTTP/1.1 201 1024 "https://aads.daouoffice.com/admin" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36" "" 0.256
203.0.113.50 DO http-nio-80-exec-45 -  [2026-08-23T02:15:32.789+0900] GET /admin/config HTTP/1.1 403 0 "http://evil.com/redirect" "python-requests/2.28.0" "" 0.012
192.168.1.100 BP http-nio-80-exec-78 user3@bizppurio.com  [2026-08-23T09:15:33.012+0900] GET /api/v1/messages/send HTTP/1.1 200 15678 "https://bizppurio.com/dashboard" "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36" "" 0.567
45.33.32.156 DO http-nio-80-exec-90 -  [2026-08-23T03:15:34.345+0900] GET /wp-admin/install.php HTTP/1.1 404 0 "-" "Mozilla/5.0 (compatible; Nmap Scripting Engine)" "" 0.001
112.220.20.130 DO http-nio-80-exec-337 user1@daouoffice.com  [2026-08-23T09:15:35.678+0900] POST /api/auth/login HTTP/1.1 200 2048 "https://aads.daouoffice.com/login" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" "" 0.890
10.0.2.33 DA http-nio-80-exec-45 user5@daouoffice.com  [2026-08-23T09:15:36.901+0900] GET /api/accounting/invoices HTTP/1.1 200 8901 "https://daouoffice.com/accounting" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" "" 0.345
172.16.0.50 DO http-nio-80-exec-22 -  [2026-08-23T09:15:37.234+0900] GET /health HTTP/1.1 200 45 "-" "curl/7.88.1" "" 0.001
89.248.167.131 DO http-nio-80-exec-15 -  [2026-08-23T04:15:38.567+0900] GET /../../etc/passwd HTTP/1.1 400 0 "-" "Go-http-client/1.1" "" 0.003
112.220.20.130 DO http-nio-80-exec-337 user1@daouoffice.com  [2026-08-23T09:15:39.890+0900] GET /api/documents/download?id=12345 HTTP/1.1 200 10485760 "https://aads.daouoffice.com/docs" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" "" 2.345
```

---

## 11. 검증 체크리스트

| # | 항목 | 검증 기준 | 상태 |
|---|------|-----------|------|
| 1 | 날짜 필드 매핑 | `@timestamp`에 모든 시간 포맷 변환 가능 | - |
| 2 | 서비스 코드 매핑 | 29개 서비스 코드 → `app.service` 변환 | - |
| 3 | IP 필드 타입 | `network.client.ip` ES ip 타입 호환 | - |
| 4 | Request 분리 | Method/Path/Version 3개 필드 분리 정상 | - |
| 5 | 상태 코드 | Short 타입 (100-599) 정상 변환 | - |
| 6 | 바이트 크기 | Long 타입 변환, 0/- 처리 | - |
| 7 | Latency 변환 | 초 → 밀리초 정수 변환 | - |
| 8 | User-Agent 파싱 | Browser/OS/Device/Bot 필드 분리 | - |
| 9 | 빈값 처리 | `-` 또는 빈 문자열 → null/기본값 변환 | - |
| 10 | 로그 레벨 | 5단계 정규화 (ERROR/WARN/INFO/DEBUG/TRACE) | - |
