-- 초기 룰셋 삽입 (MVP 10개)

-- Rule 1: Brute Force - Login
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-001-brute-force-login',
  'Brute Force - Login',
  '5분 내 동일 IP에서 로그인 실패 10회 이상 탐지',
  'high',
  true,
  'threshold',
  'count(filter(logs, log -> log.http.request.path.matches(".*/auth/login.*") && log.http.response.status_code >= 400)) >= 11',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"brute_force"}]',
  '["TA0006"]',
  '["T1110"]',
  1
);

-- Rule 2: SQL Injection
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
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
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
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
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
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
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
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
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
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
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
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
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-008-bot-scanner',
  'Bot/Scanner Traffic',
  '봇/스캐너 트래픽 다수 탐지',
  'low',
  true,
  'threshold',
  'count(filter(logs, log -> log.http.user_agent.original.matches("(?i)(bot|crawler|spider|scanner|nikto|nmap|sqlmap|dirbuster|gobuster)"))) >= 51',
  60,
  10,
  '["http.user_agent.original"]',
  '[{"type":"tag","value":"bot_traffic"}]',
  '[]',
  '[]',
  1
);

-- Rule 9: Data Exfiltration
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-009-data-exfiltration',
  'Data Exfiltration',
  '대용량 데이터 유출 의심 탐지 (10MB+)',
  'high',
  true,
  'threshold',
  'sum(filter(logs, log -> log.http.response.size >= 1), log -> log.http.response.size) >= 10485761',
  60,
  10,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"data_exfil"}]',
  '["TA0010"]',
  '["T1048"]',
  1
);

-- Rule 10: Port Scan
INSERT OR IGNORE INTO rules (id, name, description, severity, enabled, rule_type, condition, window_sec, slide_sec, group_by, actions, mitre_tactics, mitre_techniques, version)
VALUES (
  'rule-010-port-scan',
  'Port Scan / Enumeration',
  '다수 경로 탐색/열거 탐지',
  'medium',
  true,
  'threshold',
  'count(filter(logs, log -> log.http.request.path != "")) >= 11',
  300,
  60,
  '["network.client.ip"]',
  '[{"type":"alert","channel":"dashboard"},{"type":"tag","value":"port_scan"}]',
  '["TA0043"]',
  '["T1046"]',
  1
);
