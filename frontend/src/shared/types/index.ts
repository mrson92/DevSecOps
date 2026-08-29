export interface ApiResponse<T> {
  success: boolean
  data: T
  meta?: PaginationMeta
}

export interface PaginationMeta {
  page: number
  size: number
  total: number
}

export interface ApiError {
  success: boolean
  error: {
    code: string
    message: string
    details?: Record<string, unknown>
  }
}

export interface Rule {
  id: string
  name: string
  description: string | null
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info'
  enabled: boolean
  rule_type: 'threshold' | 'pattern' | 'sequence' | 'composite'
  condition: string
  window_sec: number
  slide_sec: number
  group_by: string[]
  actions: Record<string, unknown>[]
  mitre_tactics: string[]
  mitre_techniques: string[]
  references: string[]
  tags: string[]
  version: number
  parent_rule_id: string | null
  created_at: string
  updated_at: string
  created_by: string | null
  updated_by: string | null
}

export interface Detection {
  id: string
  rule_id: string
  rule_version: number
  detected_at: string
  window_start: string
  window_end: string
  matched_count: number
  group_key: string | null
  context: string | null
  status: 'open' | 'acknowledged' | 'investigating' | 'resolved' | 'false_positive' | 'suppressed'
  assignee: string | null
  created_at: string
}

export interface DashboardStats {
  total_detections: number
  open_detections: number
  critical_count: number
  high_count: number
  medium_count: number
  low_count: number
  active_rules: number
}

export interface PaginationParams {
  page?: number
  size?: number
  sort?: string
  order?: 'asc' | 'desc'
}

export interface Report {
  id: string
  report_type: string
  title: string
  period_start: string
  period_end: string
  content: string
  summary: string | null
  format: string
  status: string
  generated_at: string
}

export interface DataSource {
  id: string
  name: string
  type: string
  config: string
  target: string
  field_mapping: string
  enabled: boolean
  is_primary: boolean
  created_at: string
  updated_at: string
}

export interface NotificationChannel {
  id: string
  name: string
  type: string
  config: string
  enabled: boolean
  severity_filter: string | null
  created_at: string
}

export interface OidcSettings {
  issuer_url: string
  realm: string
  client_id: string
  client_secret: string
  redirect_url: string
  jwt_secret: string
}

export interface OidcTestResult {
  status: 'connected' | 'failed'
  message: string
  discovery?: {
    token_endpoint: string
    authorization_endpoint: string
  }
}

export interface Persona {
  id: string
  name: string
  description: string | null
  system_prompt: string
  model: string
  temperature: number
  max_tokens: number
  tools: string
  metadata: string | null
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface AiAgent {
  id: string
  name: string
  description: string | null
  persona_id: string
  agent_type: 'analyzer' | 'responder' | 'investigator' | 'reporter'
  enabled: boolean
  config: string
  schedule: string | null
  created_at: string
  updated_at: string
  created_by: string | null
  updated_by: string | null
}

export interface AiAgentRun {
  id: string
  agent_id: string
  started_at: string
  completed_at: string | null
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  input: string | null
  output: string | null
  error_message: string | null
  token_usage: number | null
  duration_ms: number | null
}

export interface CreatePersonaRequest {
  name: string
  description?: string
  system_prompt: string
  model?: string
  temperature?: number
  max_tokens?: number
  tools?: string
}

export interface UpdatePersonaRequest {
  name?: string
  description?: string
  system_prompt?: string
  model?: string
  temperature?: number
  max_tokens?: number
  tools?: string
  enabled?: boolean
}

export interface CreateAgentRequest {
  name: string
  description?: string
  persona_id: string
  agent_type?: string
  config?: string
  schedule?: string
}

export interface UpdateAgentRequest {
  name?: string
  description?: string
  persona_id?: string
  agent_type?: string
  enabled?: boolean
  config?: string
  schedule?: string
}
