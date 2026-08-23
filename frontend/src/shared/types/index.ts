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
  context: Record<string, unknown> | null
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
