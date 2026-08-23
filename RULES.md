# 초기 룰셋 상세 정의서 (MVP 10개)

## 1. 개요

| 항목 | 내용 |
|------|------|
| **버전** | 1.0 |
| **작성일** | 2026-08-23 |
| **대상** | AADS MVP 초기 룰셋 |
| **총 룰 수** | 10개 |
| **우선순위** | Critical 3개, High 4개, Medium 2개, Low 1개 |

---

## 2. 룰 상세 정의

### 룰 1: Brute Force - Login (로그인 브루트포스)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-001-brute-force-login` |
| **이름** | Brute Force - Login |
| **타입** | threshold |
| **심각도** | high |
| **윈도우** | 300초 (5분) |
| **MITRE** | T1110 (Brute Force) |

**CEL 표현식:**
```
count(filter(logs, log ->
  log.http.request.path.matches(".*/auth/login.*") &&
  log.http.response.status_code >= 400
)) > 10
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-5m" } } },
        { "wildcard": { "http.request.path": "*/auth/login*" } },
        { "range": { "http.response.status_code": { "gte": 400 } } }
      ]
    }
  },
  "aggs": {
    "by_ip": {
      "terms": { "field": "network.client.ip", "size": 100 },
      "aggs": {
        "fail_count": { "value_count": { "field": "http.response.status_code" } }
      }
    }
  },
  "size": 0
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | 동일 IP에서 5분 내 로그인 실패 11회 | 탐지 |
| 2 | 동일 IP에서 5분 내 로그인 실패 9회 | 미탐지 |
| 3 | 서로 다른 IP에서 로그인 실패 | 미탐지 (IP별 그룹핑) |
| 4 | 5분 초과하여 로그인 실패 | 미탐지 (윈도우 벗어남) |

---

### 룰 2: SQL Injection Attempt (SQL 인젝션 시도)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-002-sql-injection` |
| **이름** | SQL Injection Attempt |
| **타입** | pattern |
| **심각도** | critical |
| **윈도우** | 60초 (1분) |
| **MITRE** | T1190 (Exploit Public-Facing Application) |

**CEL 표현식:**
```
exists(logs, log ->
  log.http.request.query.matches("(?i)(union\\s+select|select\\s+.*\\s+from|insert\\s+into|update\\s+.*\\s+set|delete\\s+from|drop\\s+table|exec\\s*\\(|execute\\s+\\()") ||
  log.http.request.path.matches("(?i)(union\\s+select|select\\s+.*\\s+from|insert\\s+into|update\\s+.*\\s+set|delete\\s+from|drop\\s+table)")
)
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        {
          "bool": {
            "should": [
              { "regexp": { "http.request.query": "(?i)(union.*select|select.*from|insert.*into|update.*set|delete.*from|drop.*table|exec\\(|execute\\()" } },
              { "regexp": { "http.request.path": "(?i)(union.*select|select.*from|insert.*into|update.*set|delete.*from|drop.*table)" } }
            ]
          }
        }
      ]
    }
  },
  "size": 100
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | `GET /api/users?id=1' UNION SELECT * FROM users--` | 탐지 |
| 2 | `GET /api/search?q=test'; DROP TABLE users;--` | 탐지 |
| 3 | `POST /api/login {"username":"admin' OR '1'='1"}` | 탐지 |
| 4 | `GET /api/users?id=1` (정상 요청) | 미탐지 |
| 5 | `GET /api/users?id=1&name=test` (정상 쿼리) | 미탐지 |

---

### 룰 3: XSS Attempt (크로스 사이트 스크립팅 시도)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-003-xss-attempt` |
| **이름** | XSS Attempt |
| **타입** | pattern |
| **심각도** | high |
| **윈도우** | 60초 (1분) |
| **MITRE** | T1189 (Drive-by Compromise) |

**CEL 표현식:**
```
exists(logs, log ->
  log.http.request.query.matches("(?i)(<script|javascript:|on\\w+\\s*=|<iframe|<object|<embed|<applet|<form.*action=)") ||
  log.http.request.path.matches("(?i)(<script|javascript:|on\\w+\\s*=)")
)
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        {
          "bool": {
            "should": [
              { "regexp": { "http.request.query": "(?i)(<script|javascript:|on\\w+=|<iframe|<object|<embed|<applet)" } },
              { "regexp": { "http.request.path": "(?i)(<script|javascript:|on\\w+=)" } }
            ]
          }
        }
      ]
    }
  },
  "size": 100
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | `GET /api/search?q=<script>alert('XSS')</script>` | 탐지 |
| 2 | `GET /api/redirect?url=javascript:alert(1)` | 탐지 |
| 3 | `GET /api/page?id=1 onload=alert(1)` | 탐지 |
| 4 | `GET /api/search?q=test` (정상 검색) | 미탐지 |
| 5 | `GET /api/users?name=John` (정상 쿼리) | 미탐지 |

---

### 룰 4: Path Traversal (경로 순회)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-004-path-traversal` |
| **이름** | Path Traversal |
| **타입** | pattern |
| **심각도** | high |
| **윈도우** | 60초 (1분) |
| **MITRE** | T1083 (File and Directory Discovery) |

**CEL 표현식:**
```
exists(logs, log ->
  log.http.request.path.matches(".*\\.\\./.*") ||
  log.http.request.path.matches("(?i)(/etc/passwd|/etc/shadow|/proc/self|/windows/system32)") ||
  log.http.request.query.matches(".*\\.\\./.*")
)
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        {
          "bool": {
            "should": [
              { "regexp": { "http.request.path": ".*\\.\\./.*" } },
              { "wildcard": { "http.request.path": "*etc/passwd*" } },
              { "wildcard": { "http.request.path": "*etc/shadow*" } },
              { "regexp": { "http.request.query": ".*\\.\\./.*" } }
            ]
          }
        }
      ]
    }
  },
  "size": 100
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | `GET /api/files?name=../../../etc/passwd` | 탐지 |
| 2 | `GET /api/download?path=..%2F..%2F..%2Fetc%2Fpasswd` | 탐지 |
| 3 | `GET /api/files/read?file=....//....//etc/shadow` | 탐지 |
| 4 | `GET /api/files/list` (정상 요청) | 미탐지 |
| 5 | `GET /api/users?id=1` (정상 요청) | 미탐지 |

---

### 룰 5: Web Shell Access (웹쉘 접근)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-005-web-shell` |
| **이름** | Web Shell Access |
| **타입** | pattern |
| **심각도** | critical |
| **윈도우** | 60초 (1분) |
| **MITRE** | T1505.003 (Server Software Component: Web Shell) |

**CEL 표현식:**
```
exists(logs, log ->
  log.http.request.path.matches("(?i)(\\.php|\\.asp|\\.aspx|\\.jsp|\\.cgi|\\.pl|\\.py).*\\?.*=.*") &&
  log.http.response.status_code == 200
)
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        { "regexp": { "http.request.path": "(?i)\\.\\b(php|asp|aspx|jsp|cgi|pl|py)\\b.*\\?.*=" } },
        { "term": { "http.response.status_code": 200 } }
      ]
    }
  },
  "size": 100
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | `GET /uploads/shell.php?cmd=whoami` | 탐지 |
| 2 | `POST /admin/backdoor.asp?command=exec` | 탐지 |
| 3 | `GET /scripts/test.jsp?param=value` | 탐지 |
| 4 | `GET /index.php?page=home` (정상 PHP) | 미탐지 (파라미터 없음) |
| 5 | `GET /api/users?id=1` (정상 API) | 미탐지 |

---

### 룰 6: Privilege Escalation Attempt (권한 상승 시도)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-006-privilege-escalation` |
| **이름** | Privilege Escalation Attempt |
| **타입** | composite |
| **심각도** | critical |
| **윈도우** | 300초 (5분) |
| **MITRE** | T1068 (Exploitation for Privilege Escalation) |

**CEL 표현식:**
```
exists(logs, log ->
  log.app.user.id != null &&
  log.app.user.id != "-" &&
  log.http.request.path.matches("(?i)(/admin|/manage|/system|/config|/user.*role|/permission)") &&
  log.http.response.status_code == 200 &&
  log.network.client.ip.matches("^(?!10\\.|172\\.(1[6-9]|2[0-9]|3[01])\\.|192\\.168\\.).*")
)
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-5m" } } },
        { "exists": { "field": "app.user.id" } },
        { "regexp": { "http.request.path": "(?i)(/admin|/manage|/system|/config|/user.*role|/permission)" } },
        { "term": { "http.response.status_code": 200 } },
        { "bool": { "must_not": [
          { "cidr_match": { "network.client.ip": "10.0.0.0/8" } },
          { "cidr_match": { "network.client.ip": "172.16.0.0/12" } },
          { "cidr_match": { "network.client.ip": "192.168.0.0/16" } }
        ]}}
      ]
    }
  },
  "size": 100
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | 외부 IP에서 관리자 API 접근 (인증 후 200) | 탐지 |
| 2 | 내부 IP에서 관리자 API 접근 | 미탐지 (내부 대역) |
| 3 | 외부 IP에서 관리자 페이지 접근 (403) | 미탐지 (접근 거부) |
| 4 | 외부 IP에서 일반 API 접근 | 미탐지 (관리자 경로 아님) |

---

### 룰 7: Off-Hours Admin Access (비업무시간 관리자 접근)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-007-off-hours-admin` |
| **이름** | Off-Hours Admin Access |
| **타입** | composite |
| **심각도** | medium |
| **윈도우** | 60초 (1분) |
| **MITRE** | T1078 (Valid Accounts) |

**CEL 표현식:**
```
exists(logs, log ->
  log.http.request.path.matches("(?i)(/admin|/manage|/system|/config)") &&
  hour(log.@timestamp) >= 0 && hour(log.@timestamp) < 8
)
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        { "regexp": { "http.request.path": "(?i)(/admin|/manage|/system|/config)" } },
        {
          "script": {
            "script": {
              "source": "LocalTime.parse(doc['@timestamp'].value.format('HH:mm:ss')).getHour() >= 0 && LocalTime.parse(doc['@timestamp'].value.format('HH:mm:ss')).getHour() < 8",
              "lang": "painless"
            }
          }
        }
      ]
    }
  },
  "size": 100
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | 02:00에 관리자 페이지 접근 | 탐지 |
| 2 | 03:30에 시스템 설정 API 접근 | 탐지 |
| 3 | 10:00에 관리자 페이지 접근 | 미탐지 (업무시간) |
| 4 | 23:00에 관리자 페이지 접근 | 미탐지 (22시 이후) |

---

### 룰 8: Bot/Scanner Traffic (봇/스캐너 트래픽)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-008-bot-scanner` |
| **이름** | Bot/Scanner Traffic |
| **타입** | threshold |
| **심각도** | low |
| **윈도우** | 60초 (1분) |
| **MITRE** | - |

**CEL 표현식:**
```
count(filter(logs, log ->
  log.http.user_agent.original.matches("(?i)(bot|crawler|spider|scanner|nikto|nmap|sqlmap|dirbuster|gobuster)")
)) > 50
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        { "regexp": { "http.user_agent.original": "(?i)(bot|crawler|spider|scanner|nikto|nmap|sqlmap|dirbuster|gobuster)" } }
      ]
    }
  },
  "aggs": {
    "by_ua": {
      "terms": { "field": "http.user_agent.original", "size": 10 }
    }
  },
  "size": 0
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | 1분 내 봇 User-Agent로 100회 요청 | 탐지 |
| 2 | Nikto 스캐너로 60회 요청 | 탐지 |
| 3 | 일반 브라우저로 100회 요청 | 미탐지 |
| 4 | 봇 User-Agent로 30회 요청 | 미탐지 (임계값 미달) |

---

### 룰 9: Data Exfiltration (데이터 유출 의심)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-009-data-exfiltration` |
| **이름** | Data Exfiltration |
| **타입** | threshold |
| **심각도** | high |
| **윈도우** | 60초 (1분) |
| **MITRE** | T1048 (Exfiltration Over Alternative Protocol) |

**CEL 표현식:**
```
sum(filter(logs, log -> log.http.response.size), log -> log.http.response.size) > 10485760
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-1m" } } },
        { "range": { "http.response.size": { "gte": 10485760 } } }
      ]
    }
  },
  "aggs": {
    "by_ip": {
      "terms": { "field": "network.client.ip", "size": 100 },
      "aggs": {
        "total_bytes": { "sum": { "field": "http.response.size" } }
      }
    }
  },
  "size": 0
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | 1분 내 동일 IP에서 15MB 응답 데이터 수신 | 탐지 |
| 2 | 1분 내 동일 IP에서 8MB 응답 데이터 수신 | 미탐지 |
| 3 | 서로 다른 IP에서 5MB씩 수신 | 미탐지 (IP별 집계) |
| 4 | 1분 초과하여 15MB 수신 | 미탐지 (윈도우 벗어남) |

---

### 룰 10: Port Scan / Enumeration (포트 스캔/열거)

| 항목 | 내용 |
|------|------|
| **ID** | `rule-010-port-scan` |
| **이름** | Port Scan / Enumeration |
| **타입** | threshold |
| **심각도** | medium |
| **윈도우** | 300초 (5분) |
| **MITRE** | T1046 (Network Service Discovery) |

**CEL 표현식:**
```
count(distinct(filter(logs, log -> log.http.request.path), log -> log.http.request.path)) > 10
```

**ES 쿼리:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "range": { "@timestamp": { "gte": "now-5m" } } }
      ]
    }
  },
  "aggs": {
    "by_ip": {
      "terms": { "field": "network.client.ip", "size": 100 },
      "aggs": {
        "unique_paths": { "cardinality": { "field": "http.request.path" } }
      }
    }
  },
  "size": 0
}
```

**테스트케이스:**
| # | 시나리오 | 예상 결과 |
|---|----------|-----------|
| 1 | 5분 내 동일 IP에서 15개 다른 경로 접근 | 탐지 |
| 2 | 5분 내 동일 IP에서 8개 다른 경로 접근 | 미탐지 |
| 3 | 서로 다른 IP에서 경로 접근 | 미탐지 (IP별 그룹핑) |
| 4 | 5분 초과하여 15개 경로 접근 | 미탐지 (윈도우 벗어남) |

---

## 3. 초기 룰 SQL 삽입 스크립트

```sql
-- Rule 1: Brute Force - Login
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-001-brute-force-login',
  'Brute Force - Login',
  '5분 내 동일 IP에서 로그인 실패 10회 이상 탐지',
  'high',
  true,
  'threshold',
  'count(filter(logs, log -> log.http.request.path.matches(".*/auth/login.*") && log.http.response.status_code >= 400)) > 10',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"brute_force"}]',
  '["TA0006"]',
  '["T1110"]',
  1
);

-- Rule 2: SQL Injection
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-002-sql-injection',
  'SQL Injection Attempt',
  'SQL 인젝션 패턴 매칭 탐지',
  'critical',
  true,
  'pattern',
  'exists(logs, log -> log.http.request.query.matches("(?i)(union.*select|select.*from|insert.*into|update.*set|delete.*from|drop.*table|exec\\(|execute\\()"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"sqli"}]',
  '["TA0001"]',
  '["T1190"]',
  1
);

-- Rule 3: XSS
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-003-xss-attempt',
  'XSS Attempt',
  '크로스 사이트 스크립팅 패턴 매칭 탐지',
  'high',
  true,
  'pattern',
  'exists(logs, log -> log.http.request.query.matches("(?i)(<script|javascript:|on\\w+\\s*=|<iframe|<object|<embed|<applet)"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"xss"}]',
  '["TA0001"]',
  '["T1189"]',
  1
);

-- Rule 4: Path Traversal
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-004-path-traversal',
  'Path Traversal',
  '경로 순회 패턴 매칭 탐지',
  'high',
  true,
  'pattern',
  'exists(logs, log -> log.http.request.path.matches(".*\\.\\./.*") || log.http.request.path.matches("(?i)(/etc/passwd|/etc/shadow|/proc/self|/windows/system32)"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"path_traversal"}]',
  '["TA0007"]',
  '["T1083"]',
  1
);

-- Rule 5: Web Shell
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-005-web-shell',
  'Web Shell Access',
  '웹쉘 파일 접근 패턴 매칭 탐지',
  'critical',
  true,
  'pattern',
  'exists(logs, log -> log.http.request.path.matches("(?i)(\\.php|\\.asp|\\.aspx|\\.jsp|\\.cgi|\\.pl|\\.py).*\\?.*=.*") && log.http.response.status_code == 200)',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"web_shell"}]',
  '["TA0003"]',
  '["T1505.003"]',
  1
);

-- Rule 6: Privilege Escalation
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-006-privilege-escalation',
  'Privilege Escalation Attempt',
  '외부 IP에서 관리자 경로 접근 탐지',
  'critical',
  true,
  'composite',
  'exists(logs, log -> log.app.user.id != null && log.http.request.path.matches("(?i)(/admin|/manage|/system|/config|/user.*role|/permission)") && log.http.response.status_code == 200 && !log.network.client.ip.matches("^(10\\.|172\\.(1[6-9]|2[0-9]|3[01])\\.|192\\.168\\.).*"))',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"priv_escalation"}]',
  '["TA0004"]',
  '["T1068"]',
  1
);

-- Rule 7: Off-Hours Admin
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-007-off-hours-admin',
  'Off-Hours Admin Access',
  '비업무시간(00-08시) 관리자 접근 탐지',
  'medium',
  true,
  'composite',
  'exists(logs, log -> log.http.request.path.matches("(?i)(/admin|/manage|/system|/config)") && hour(log.@timestamp) >= 0 && hour(log.@timestamp) < 8)',
  60,
  10,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"off_hours_admin"}]',
  '["TA0007"]',
  '["T1078"]',
  1
);

-- Rule 8: Bot/Scanner
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-008-bot-scanner',
  'Bot/Scanner Traffic',
  '봇/스캐너 트래픽 다수 탐지',
  'low',
  true,
  'threshold',
  'count(filter(logs, log -> log.http.user_agent.original.matches("(?i)(bot|crawler|spider|scanner|nikto|nmap|sqlmap|dirbuster|gobuster)"))) > 50',
  60,
  10,
  '["http.user_agent.original"]',
  '[{"type":"tag","value":"bot_traffic"}]',
  '[]',
  '[]',
  1
);

-- Rule 9: Data Exfiltration
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-009-data-exfiltration',
  'Data Exfiltration',
  '대용량 데이터 유출 의심 탐지 (10MB+)',
  'high',
  true,
  'threshold',
  'sum(filter(logs, log -> log.http.response.size > 0), log -> log.http.response.size) > 10485760',
  60,
  10,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"data_exfil"}]',
  '["TA0010"]',
  '["T1048"]',
  1
);

-- Rule 10: Port Scan
INSERT INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-010-port-scan',
  'Port Scan / Enumeration',
  '다수 경로 탐색/열거 탐지',
  'medium',
  true,
  'threshold',
  'count(distinct(filter(logs, log -> true), log -> log.http.request.path)) > 10',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"port_scan"}]',
  '["TA0043"]',
  '["T1046"]',
  1
);
```

---

## 4. 검증 체크리스트

| # | 항목 | 검증 기준 | 상태 |
|---|------|-----------|------|
| 1 | CEL 문법 검증 | 10개 룰 모두 CEL 컴파일 가능 | - |
| 2 | ES 쿼리 검증 | 10개 룰 모두 ES 8.x에서 실행 가능 | - |
| 3 | 그룹핑 검증 | group_by 필드가 ES mapping에 존재 | - |
| 4 | 윈도우 검증 | window_sec/slide_sec 유효한 값 | - |
| 5 | 심각도 검증 | severity가 정의된 값만 사용 | - |
| 6 | MITRE 매핑 | ATT&CK 기법 ID 정확성 | - |
| 7 | 테스트케이스 | 각 룰 최소 4개 시나리오 | - |
| 8 | SQL 삽입 | 10개 룰 DB 삽입 가능 | - |
