-- External-signature-inspired MITRE-mapped rules (batch 2)
-- Designed for the native evaluator DSL: short field names (path, query, method,
-- status_code, response_size, response_time, client_ip, user_agent.original, user_id,
-- source, timestamp), regex via .matches("..."), comparisons via >= / > / < / <= / == / !=,
-- has("field"), and char-class bracket regexes to avoid backslash-escaping pitfalls.
-- All rules are idempotent (INSERT OR IGNORE).
--
-- Unlocks previously-empty MITRE groups: TA0002 (Execution), TA0005 (Defense Evasion),
-- TA0011 (Command & Control) -- and expands TA0003, TA0006, TA0007, TA0010.

-- 011 Command Injection (Execution) -> TA0002 / T1059
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-011-command-injection',
  'Command Injection Attempt',
  '쿼리/파라미터에 셸 명령 실행 시도 패턴 탐지',
  'critical',
  true,
  'pattern',
  'exists(logs, log -> query.matches("(?i)[;|&][[:space:]]*(wget|curl|nc|ncat|netcat|bash|sh|/bin/sh|powershell|cmd|python|perl|ruby|php -r|id|whoami|uname|cat|rm|chmod|chown|mkfifo|base64|apt|yum|scp|socat)"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"cmd_injection"}]',
  '["TA0002"]',
  '["T1059"]',
  1
);

-- 012 Encoded Command Execution (Execution) -> TA0002 / T1059
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-012-encoded-command-execution',
  'Encoded Command Execution',
  '인코딩된 셸/파워쉘 명령 실행 탐지 (base64 등)',
  'high',
  true,
  'pattern',
  'exists(logs, log -> query.matches("(?i)((base64|b64|enc|encoded|enc64|eval|deobfuscate)[=_-][^&]{4,}.*(powershell|cmd|sh|bash|echo)|(LmV4ZQ==|a2F0|Y2F0IC9ldGMv|aHR0cDov|cm0gLWZ|aGxl)))"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"encoded_cmd"}]',
  '["TA0002"]',
  '["T1059"]',
  1
);

-- 013 Suspicious Encoding / Obfuscated Payload (Defense Evasion) -> TA0005 / T1036
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-013-suspicious-encoding',
  'Suspicious Payload Encoding',
  '과도한 인코딩/난독화된 페이로드 패턴 탐지',
  'medium',
  true,
  'pattern',
  'exists(logs, log -> query.matches("(?i)(%[0-9a-f]{2}%[0-9a-f]{2}%[0-9a-f]{2}|(?:u%[0-9a-f]{4}){2,}|[?&](data|payload|cmd|exec|p|q)[=][A-Za-z0-9+/]{24,}={0,2}|[?&][A-Za-z0-9_]+[=](?:[A-Za-z0-9+/]{4}){12,}={0,2})"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"obfuscation"}]',
  '["TA0005"]',
  '["T1036"]',
  1
);

-- 014 Credential Dump / Sensitive File Probe (Credential Access) -> TA0006 / T1003
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-014-credential-dump',
  'Credential Dump / Sensitive File Access',
  '자격증명/민감 설정 파일 접근 패턴 탐지',
  'high',
  true,
  'pattern',
  'exists(logs, log -> path.matches("(?i)(/etc/passwd|/etc/shadow|/proc/[0-9]+/(environ|mem|cmdline)|/home/[a-z0-9_]+/[.]ssh|/[.]aws/credentials|/[.]env|/[.]htpasswd|/[.]git/config|/wp-config[.]php|/web[.]config|/database[.](sql|bak|dump|sqlite)|/[.]kube/config)"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"cred_dump"}]',
  '["TA0006"]',
  '["T1003"]',
  1
);

-- 015 Password Spraying (Credential Access) -> TA0006 / T1110.003
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-015-password-spraying',
  'Password Spraying',
  '저빈도 로그인 실패로 다수 계정 시도 (스프레이) 탐지',
  'high',
  true,
  'threshold',
  'count(filter(logs, log -> path.matches("(?i)(/auth/login|/login|/signin|/oauth/token|/api/v[0-9]+/auth/login)") && method == "POST" && status_code >= 401 && status_code <= 429)) >= 5',
  300,
  60,
  '["network.client.ip", "http.user_agent.original"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"password_spray"}]',
  '["TA0006"]',
  '["T1110.003"]',
  1
);

-- 016 New Account Creation (Persistence) -> TA0003 / T1136
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-016-account-creation',
  'Suspicious Account Creation',
  '회원가입/계정 생성 요청 다수 탐지 (지속성)',
  'medium',
  true,
  'threshold',
  'count(filter(logs, log -> path.matches("(?i)(/register|/signup|/api/v[0-9]+/(users|accounts)|/admin/(users|accounts))") && method == "POST" && status_code >= 200 && status_code <= 204)) >= 3',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"account_creation"}]',
  '["TA0003"]',
  '["T1136"]',
  1
);

-- 017 Sensitive Path/Config Discovery (Discovery) -> TA0007 / T1083
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-017-suspicious-discovery',
  'Suspicious Path & Config Discovery',
  '관리자/설정/백업 등 민감 경로 탐색 탐지',
  'medium',
  true,
  'pattern',
  'exists(logs, log -> path.matches("(?i)(/admin|/dashboard|/wp-admin|/phpmyadmin|/server-status|/server-info|/config|/backup|/logs|/uploads|/[.]git/|/[.]env|/[.]aws/|/[.]ssh/|/robots.txt)"))',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"discovery"}]',
  '["TA0007"]',
  '["T1083"]',
  1
);

-- 018 Archive / Backup Exfiltration (Exfiltration) -> TA0010 / T1567
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-018-archive-exfiltration',
  'Sensitive Archive / Backup Download',
  '아카이브/백업/데이터베이스 대용량 다운로드 탐지',
  'high',
  true,
  'threshold',
  'count(filter(logs, log -> path.matches("(?i)/[^?]*[.](zip|rar|7z|tar[.]gz|tar[.]bz2|tar|gz|sql|bak|dump|db|pem|key|pst|ost)($|[?])") && response_size >= 1048576)) >= 1',
  60,
  10,
  '["network.client.ip", "http.request.path"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"archive_exfil"}]',
  '["TA0010"]',
  '["T1567"]',
  1
);

-- 019 Suspicious C2 / Tool User-Agent (Command & Control) -> TA0011 / T1071
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-019-c2-user-agent',
  'Suspicious C2 / Tool User-Agent',
  '알려진 C2/공격 도구 사용자 에이전트 탐지',
  'high',
  true,
  'pattern',
  'exists(logs, log -> user_agent.original.matches("(?i)(cobalt|beacon|mimikatz|metasploit|meterpreter|empire|nishang|psexec|sliver|havoc|brute ratel|evilginx|wmiexec|responder|sqlmap|nmap|gobuster|nikto|dirbuster)"))',
  60,
  10,
  '["network.client.ip", "http.user_agent.original"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"c2_tool"}]',
  '["TA0011"]',
  '["T1071"]',
  1
);

-- 020 Abusive HTTP Method Probing (Discovery) -> TA0007 / T1046
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-020-method-probing',
  'Abusive / Probing HTTP Method Scan',
  'TRACE/OPTIONS/PUT/DELETE 등 비정상 HTTP 메서드 다수 탐지',
  'medium',
  true,
  'threshold',
  'count(filter(logs, log -> method.matches("(?i)(TRACE|OPTIONS|CONNECT|PROPFIND|MKCOL|COPY|MOVE)"))) >= 5',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"method_probe"}]',
  '["TA0007"]',
  '["T1046"]',
  1
);
