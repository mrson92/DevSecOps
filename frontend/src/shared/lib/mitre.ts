export function parseStringArray(value: string[] | string | null | undefined): string[] {
  if (!value) return []
  if (Array.isArray(value)) return value
  try {
    const parsed = JSON.parse(value)
    return Array.isArray(parsed) ? parsed.map(String) : []
  } catch {
    return []
  }
}

export const MITRE_TACTICS: Record<string, string> = {
  TA0001: 'Initial Access',
  TA0002: 'Execution',
  TA0003: 'Persistence',
  TA0004: 'Privilege Escalation',
  TA0005: 'Defense Evasion',
  TA0006: 'Credential Access',
  TA0007: 'Discovery',
  TA0008: 'Lateral Movement',
  TA0009: 'Collection',
  TA0010: 'Exfiltration',
  TA0011: 'Command and Control',
  TA0040: 'Impact',
}

export const MITRE_TECHNIQUES: Record<string, string> = {
  T1110: 'Brute Force',
  T1078: 'Valid Accounts',
  T1190: 'Exploit Public-Facing Application',
  T1189: 'Drive-by Compromise',
  T1083: 'File and Directory Discovery',
  'T1505.003': 'Web Shell',
  T1059: 'Command and Scripting Interpreter',
  T1046: 'Network Service Discovery',
  T1036: 'Masquerading',
  T1548: 'Abuse Elevation Control Mechanism',
  T1003: 'OS Credential Dumping',
  T1041: 'Exfiltration Over C2 Channel',
  T1567: 'Exfiltration Over Web Service',
  T1571: 'Non-Standard Port',
  T1071: 'Application Layer Protocol',
  T1136: 'Create Account',
  T1566: 'Phishing',
  T1040: 'Network Sniffing',
  T1505: 'Server Software Component',
  T1622: 'Debugger Evasion',
}

export function tacticName(id: string): string {
  return MITRE_TACTICS[id] ?? id
}

export function techniqueName(id: string): string {
  return MITRE_TECHNIQUES[id] ?? id
}
