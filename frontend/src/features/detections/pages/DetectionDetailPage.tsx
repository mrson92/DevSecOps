import { useMemo, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Detection, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { useRuleName } from '../hooks/useRuleName'

type EvidenceItem = Record<string, unknown>

interface ParsedEvidence {
  ok: boolean
  items: EvidenceItem[]
}

function parseEvidence(context: string | null): ParsedEvidence {
  if (!context || context.trim() === '') {
    return { ok: true, items: [] }
  }
  try {
    const value: unknown = JSON.parse(context)
    if (Array.isArray(value)) return { ok: true, items: value as EvidenceItem[] }
    if (value && typeof value === 'object') return { ok: true, items: [value as EvidenceItem] }
    return { ok: true, items: [] }
  } catch {
    return { ok: false, items: [] }
  }
}

const FIELD_ORDER = [
  'timestamp', 'source', 'client_ip', 'method', 'path', 'query', 'status_code',
  'response_size', 'user_agent', 'user_id', 'response_time', 'extra',
]

const FIELD_LABELS: Record<string, string> = {
  timestamp: 'Timestamp',
  source: 'Source',
  client_ip: 'Client IP',
  method: 'Method',
  path: 'Path',
  query: 'Query',
  status_code: 'Status Code',
  response_size: 'Response Size',
  user_agent: 'User Agent',
  user_id: 'User ID',
  response_time: 'Response Time',
  extra: 'Extra',
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

function formatTimestamp(value: string): string {
  const d = new Date(value)
  return Number.isNaN(d.getTime()) ? value : d.toLocaleString()
}

function statusBadge(value: unknown) {
  if (typeof value === 'string') value = Number(value)
  if (typeof value !== 'number' || Number.isNaN(value)) return null
  const className =
    value >= 500 ? 'bg-red-100 text-red-800' :
    value >= 400 ? 'bg-orange-100 text-orange-800' :
    value >= 300 ? 'bg-yellow-100 text-yellow-800' :
    'bg-slate-100 text-slate-800'
  return (
    <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${className}`}>
      {value}
    </span>
  )
}

function EvidenceCard({
  index,
  total,
  item,
}: {
  index: number
  total: number
  item: EvidenceItem
}) {
  const [expanded, setExpanded] = useState(false)
  const timestamp = typeof item.timestamp === 'string' ? formatTimestamp(item.timestamp) : ''
  const source = formatValue(item.source)
  const clientIp = formatValue(item.client_ip)
  const method = formatValue(item.method)
  const pathEl = formatValue(item.path)
  const query = formatValue(item.query)

  const entries = Object.entries(item)
    .filter(([, v]) => v !== null && v !== undefined && v !== '')
    .sort((a, b) => {
      const ia = FIELD_ORDER.indexOf(a[0])
      const ib = FIELD_ORDER.indexOf(b[0])
      if (ia === -1 && ib === -1) return a[0].localeCompare(b[0])
      if (ia === -1) return 1
      if (ib === -1) return -1
      return ia - ib
    })

  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-medium">
              {total > 1 && (
                <span className="text-muted-foreground font-normal">
                  Evidence #{index}/{total}:{' '}
                </span>
              )}
              {method && (
                <span className="px-1.5 py-0.5 text-xs bg-slate-100 rounded mr-1.5">{method}</span>
              )}
              {pathEl && (
                <span className="font-mono text-xs break-all">
                  {pathEl}
                  {query ? <span className="text-muted-foreground">?{query}</span> : null}
                </span>
              )}
            </div>
            <div className="mt-1.5 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
              {timestamp && <span>{timestamp}</span>}
              {source && <span>{source}</span>}
              {clientIp && <span className="font-mono">{clientIp}</span>}
            </div>
          </div>
          {statusBadge(item.status_code)}
        </div>
        {entries.length > 0 && (
          <div className="mt-3">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs -ml-2"
              onClick={() => setExpanded((v) => !v)}
            >
              {expanded ? 'Hide details' : 'Show details'}
            </Button>
            {expanded && (
              <div className="mt-2 pt-3 border-t grid gap-x-6 gap-y-2 sm:grid-cols-2">
                {entries.map(([key, value]) => (
                  <div key={key} className="flex flex-col gap-0.5 min-w-0">
                    <span className="text-xs font-medium text-muted-foreground">
                      {FIELD_LABELS[key] ?? key}
                    </span>
                    <span className="text-xs font-mono break-all whitespace-pre-wrap">
                      {formatValue(value)}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function EvidenceSection({ context }: { context: string | null }) {
  const { ok, items } = useMemo(() => parseEvidence(context), [context])

  if (!ok) {
    return (
      <pre className="p-4 bg-muted rounded-md overflow-x-auto text-sm">
        {context}
      </pre>
    )
  }

  if (items.length === 0) {
    return <p className="text-sm text-muted-foreground">No evidence available</p>
  }

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        {items.length} logged {items.length === 1 ? 'entry' : 'entries'} matched this rule
      </p>
      {items.map((item, idx) => (
        <EvidenceCard key={idx} index={idx + 1} total={items.length} item={item} />
      ))}
    </div>
  )
}

export function DetectionDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { byId } = useRuleName()

  const { data: detectionData, isLoading } = useQuery<ApiResponse<Detection>>({
    queryKey: ['detection', id],
    queryFn: () => api.get(`/detections/${id}`).then((res) => res.data),
    enabled: !!id,
  })

  const updateStatusMutation = useMutation({
    mutationFn: (status: string) =>
      api.patch(`/detections/${id}`, { status }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['detection', id] })
      queryClient.invalidateQueries({ queryKey: ['detections'] })
    },
  })

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  const detection = detectionData?.data

  if (!detection) {
    return <div className="text-center py-8">Detection not found</div>
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Detection #{detection.id.slice(0, 8)}</h1>
          <p className="text-muted-foreground">
            Detected at {new Date(detection.detected_at).toLocaleString()}
          </p>
          <p className="text-sm">
            Rule:{' '}
            <button
              className="font-medium text-primary underline underline-offset-4"
              onClick={() => navigate(`/rules/${detection.rule_id}`)}
            >
              {byId(detection.rule_id)?.name ?? detection.rule_id.slice(0, 8)}
            </button>
          </p>
        </div>
        <div className="flex gap-2">
          {detection.status === 'open' && (
            <Button
              variant="outline"
              onClick={() => updateStatusMutation.mutate('acknowledged')}
              disabled={updateStatusMutation.isPending}
            >
              Acknowledge
            </Button>
          )}
          {(detection.status === 'open' || detection.status === 'acknowledged' || detection.status === 'investigating') && (
            <Button
              onClick={() => updateStatusMutation.mutate('resolved')}
              disabled={updateStatusMutation.isPending}
            >
              Resolve
            </Button>
          )}
          <Button
            variant="ghost"
            onClick={() => navigate('/detections')}
          >
            Back
          </Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Status</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`px-2 py-1 text-xs rounded-full ${
              detection.status === 'open' ? 'bg-red-100 text-red-800' :
              detection.status === 'acknowledged' ? 'bg-yellow-100 text-yellow-800' :
              detection.status === 'investigating' ? 'bg-blue-100 text-blue-800' :
              'bg-green-100 text-green-800'
            }`}>
              {detection.status}
            </span>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Matched Count</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{detection.matched_count}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Rule</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1">
            <button
              className="text-sm font-semibold text-primary underline underline-offset-4"
              onClick={() => navigate(`/rules/${detection.rule_id}`)}
            >
              {byId(detection.rule_id)?.name ?? detection.rule_id.slice(0, 8)}
            </button>
            <div className="text-xs text-muted-foreground">v{detection.rule_version}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Assignee</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{detection.assignee ?? 'Unassigned'}</div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Window</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm">
            <div><span className="text-muted-foreground">Start:</span> {new Date(detection.window_start).toLocaleString()}</div>
            <div><span className="text-muted-foreground">End:</span> {new Date(detection.window_end).toLocaleString()}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Group Key</CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="p-3 bg-muted rounded-md overflow-x-auto text-xs">
              {detection.group_key ?? 'N/A'}
            </pre>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Evidence</CardTitle>
        </CardHeader>
        <CardContent>
          <EvidenceSection context={detection.context} />
        </CardContent>
      </Card>
    </div>
  )
}
