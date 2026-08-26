-- Fix seed rules: use short field names that our native evaluator understands
-- Fields: path, query, method, status_code, response_size, client_ip, user_agent, user_id

-- Rule 1: Brute Force - Login (count HTTP status >= 400 on /auth/login)
UPDATE rules SET condition = 'count(filter(logs, log -> path.matches(".*/auth/login.*") && status_code >= 400)) >= 11' WHERE id = 'rule-001-brute-force-login';

-- Rule 2: SQL Injection (regex on query field)
UPDATE rules SET condition = 'exists(logs, log -> query.matches("(?i)(union.*select|select.*from|insert.*into|update.*set|delete.*from|drop.*table|exec\\\\(|execute\\\\()"))' WHERE id = 'rule-002-sql-injection';

-- Rule 3: XSS (regex on query field)
UPDATE rules SET condition = 'exists(logs, log -> query.matches("(?i)(<script|javascript:|on\\\\w+\\\\s*=|<iframe|<object|<embed|<applet)"))' WHERE id = 'rule-003-xss-attempt';

-- Rule 4: Path Traversal (regex on path field)
UPDATE rules SET condition = 'exists(logs, log -> path.matches(".*\\\\.\\\\./.*") || path.matches("(?i)(/etc/passwd|/etc/shadow|/proc/self|/windows/system32)"))' WHERE id = 'rule-004-path-traversal';

-- Rule 5: Web Shell (regex on path + status_code == 200)
UPDATE rules SET condition = 'exists(logs, log -> path.matches("(?i)(\\\\.php|\\\\.asp|\\\\.aspx|\\\\.jsp|\\\\.cgi|\\\\.pl|\\\\.py).*\\\\?.*=.*") && status_code == 200)' WHERE id = 'rule-005-web-shell';

-- Rule 6: Privilege Escalation (user_id present + regex on path + status 200 + not private IP)
UPDATE rules SET condition = 'exists(logs, log -> has("user_id") && path.matches("(?i)(/admin|/manage|/system|/config|/user.*role|/permission)") && status_code == 200 && !client_ip.matches("^(10\\\\.|172\\\\.(1[6-9]|2[0-9]|3[01])\\\\.|192\\\\.168\\\\.).*"))' WHERE id = 'rule-006-privilege-escalation';

-- Rule 7: Off-Hours Admin (regex on path)
UPDATE rules SET condition = 'exists(logs, log -> path.matches("(?i)(/admin|/manage|/system|/config)"))' WHERE id = 'rule-007-off-hours-admin';

-- Rule 8: Bot/Scanner (regex on user_agent)
UPDATE rules SET condition = 'count(filter(logs, log -> user_agent.original.matches("(?i)(bot|crawler|spider|scanner|nikto|nmap|sqlmap|dirbuster|gobuster)"))) >= 51' WHERE id = 'rule-008-bot-scanner';

-- Rule 9: Data Exfiltration (response size >= 10MB)
UPDATE rules SET condition = 'count(filter(logs, log -> response_size >= 10485761)) >= 1' WHERE id = 'rule-009-data-exfiltration';

-- Rule 10: Port Scan (distinct paths >= 11)
UPDATE rules SET condition = 'count(filter(logs, log -> path != "")) >= 11' WHERE id = 'rule-010-port-scan';
