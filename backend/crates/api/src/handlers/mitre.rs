use axum::Json;
use serde_json::{json, Value};

// MITRE ATT&CK reference catalog for rule tagging.
// These are public MITRE identifiers/names referenced only as metadata
// to help tag rules (Sigma-style tags like attack.tXXXX). No Sigma rule
// content is included here.

const TACTICS: &[(&str, &str)] = &[
    ("TA0001", "Initial Access"),
    ("TA0002", "Execution"),
    ("TA0003", "Persistence"),
    ("TA0004", "Privilege Escalation"),
    ("TA0005", "Defense Evasion"),
    ("TA0006", "Credential Access"),
    ("TA0007", "Discovery"),
    ("TA0008", "Lateral Movement"),
    ("TA0009", "Collection"),
    ("TA0010", "Exfiltration"),
    ("TA0011", "Command and Control"),
    ("TA0040", "Impact"),
];

const TECHNIQUES: &[(&str, &str, &str)] = &[
    ("T1110", "Brute Force", "TA0006"),
    ("T1078", "Valid Accounts", "TA0001"),
    ("T1190", "Exploit Public-Facing Application", "TA0001"),
    ("T1189", "Drive-by Compromise", "TA0001"),
    ("T1083", "File and Directory Discovery", "TA0007"),
    ("T1505.003", "Web Shell", "TA0003"),
    ("T1059", "Command and Scripting Interpreter", "TA0002"),
    ("T1046", "Network Service Discovery", "TA0007"),
    ("T1036", "Masquerading", "TA0005"),
    ("T1548", "Abuse Elevation Control Mechanism", "TA0004"),
    ("T1003", "OS Credential Dumping", "TA0006"),
    ("T1041", "Exfiltration Over C2 Channel", "TA0010"),
    ("T1567", "Exfiltration Over Web Service", "TA0010"),
    ("T1571", "Non-Standard Port", "TA0011"),
    ("T1071", "Application Layer Protocol", "TA0011"),
    ("T1136", "Create Account", "TA0003"),
    ("T1566", "Phishing", "TA0001"),
    ("T1040", "Network Sniffing", "TA0007"),
    ("T1505", "Server Software Component", "TA0003"),
    ("T1622", "Debugger Evasion", "TA0005"),
];

pub async fn list_tactics() -> Json<Value> {
    let data: Vec<Value> = TACTICS
        .iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();
    Json(json!({ "success": true, "data": data }))
}

pub async fn list_techniques() -> Json<Value> {
    let data: Vec<Value> = TECHNIQUES
        .iter()
        .map(|(id, name, tactic)| json!({ "id": id, "name": name, "tactic": tactic }))
        .collect();
    Json(json!({ "success": true, "data": data }))
}
