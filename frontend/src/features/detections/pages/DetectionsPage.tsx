import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import api from '@/shared/lib/api'
import type { Detection, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { MitreBadges } from '@/shared/components/ui/MitreBadges'
import { MITRE_TACTICS } from '@/shared/lib/mitre'
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts'
import { useRuleName } from '../hooks/useRuleName'

const RANGE_PRESETS = [
  { id: '1d', label: 'Last 24 hours', hours: 24 },
  { id: '7d', label: 'Last 7 days', hours: 168 },
  { id: '30d', label: 'Last 30 days', hours: 720 },
] as const

const STATUS_STYLES: Record<string, string> = {
  open: 'bg-red-100 text-red-800',
  acknowledged: 'bg-yellow-100 text-yellow-800',
  investigating: 'bg-blue-100 text-blue-800',
  resolved: 'bg-green-100 text-green-800',
  false_positive: 'bg-gray-100 text-gray-800',
  suppressed: 'bg-gray-100 text-gray-700',
}

const SEVERITY_ORDER = ['critical', 'high', 'medium', 'low', 'info'] as const
const SEVERITY_STYLES: Record<string, string> = {
  critical: 'bg-red-600 text-white',
  high: 'bg-orange-500 text-white',
  medium: 'bg-yellow-400 text-black',
  low: 'bg-blue-500 text-white',
  info: 'bg-slate-400 text-white',
}

function fmtDate(ts: string): string {
  const d = new Date(ts)
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString('ko-KR')
}

interface DetectionFilters {
  severity?: string
  status?: string
  tactic?: string
  page: number
  size: number
  range: string
}

interface HistogramBucket {
  ts: number
  count: number
}

export function DetectionsPage() {
  const [filters, setFilters] = useState<DetectionFilters>({
    page: 1,
    size: 20,
    range: '7d',
  })
  const [showFilters, setShowFilters] = useState(false)
  const { byId } = useRuleName()

  const { data: detectionsData, isLoading } = useQuery<ApiResponse<Detection[]>>({
    queryKey: ['detections', filters],
    queryFn: () => {
      const params = new URLSearchParams()
      params.set('page', String(filters.page))
      params.set('size', String(filters.size))
      if (filters.severity) params.set('severity', filters.severity)
      if (filters.status) params.set('status', filters.status)
      if (filters.tactic) params.set('tactic', filters.tactic)
      return api.get(`/detections?${params.toString()}`).then((res) => res.data)
    },
  })

  const range = RANGE_PRESETS.find((p) => p.id === filters.range) ?? RANGE_PRESETS[1]
  const histogramRange = useMemo(() => {
    const to = new Date()
    const from = new Date(to.getTime() - range.hours * 3600_000)
    return { from: from.toISOString(), to: to.toISOString() }
  }, [range])

  const { data: histData, isFetching: isHistFetching } = useQuery<ApiResponse<HistogramBucket[]>>({
    queryKey: ['detections-histogram', filters.range, filters.severity, filters.status, filters.tactic],
    queryFn: () => {
      const params = new URLSearchParams()
      params.set('start_date', histogramRange.from)
      params.set('end_date', histogramRange.to)
      const hours = range.hours
      const interval = hours <= 24 ? 3600 : hours <= 168 ? 4 * 3600 : 24 * 3600
      params.set('interval', String(interval))
      if (filters.severity) params.set('severity', filters.severity)
      if (filters.status) params.set('status', filters.status)
      if (filters.tactic) params.set('tactic', filters.tactic)
      return api.get(`/detections/histogram?${params.toString()}`).then((res) => res.data)
    },
  })

  const histogram = (histData?.data ?? []).map((b) => ({
    ts: b.ts,
    label: new Date(b.ts * 1000).toLocaleTimeString('ko-KR', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    }),
    count: b.count,
  }))

  const maxCount = histogram.length > 0 ? Math.max(...histogram.map((h) => h.count)) : 0

  const detections = detectionsData?.data ?? []
  const meta = detectionsData?.meta
  const totalPages = meta ? Math.ceil(meta.total / filters.size) : 1

  const updateFilter = (key: keyof DetectionFilters, value: string | number | undefined) => {
    setFilters((prev) => ({ ...prev, [key]: value, page: 1 }))
  }

  const sorted = [...detections].sort((a, b) => {
    const ra = byId(a.rule_id)
    const rb = byId(b.rule_id)
    const sa = ra ? SEVERITY_ORDER.indexOf(ra.severity as (typeof SEVERITY_ORDER)[number]) : SEVERITY_ORDER.length
    const sb = rb ? SEVERITY_ORDER.indexOf(rb.severity as (typeof SEVERITY_ORDER)[number]) : SEVERITY_ORDER.length
    return sa - sb
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Detections</h1>
        <Button variant="outline" onClick={() => setShowFilters(!showFilters)}>
          {showFilters ? 'Hide Filters' : 'Show Filters'}
        </Button>
      </div>

      {showFilters && (
        <Card>
          <CardContent className="py-4">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Severity</label>
                <select
                  value={filters.severity ?? ''}
                  onChange={(e) => updateFilter('severity', e.target.value || undefined)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                >
                  <option value="">All</option>
                  <option value="critical">Critical</option>
                  <option value="high">High</option>
                  <option value="medium">Medium</option>
                  <option value="low">Low</option>
                </select>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Status</label>
                <select
                  value={filters.status ?? ''}
                  onChange={(e) => updateFilter('status', e.target.value || undefined)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                >
                  <option value="">All</option>
                  <option value="open">Open</option>
                  <option value="acknowledged">Acknowledged</option>
                  <option value="investigating">Investigating</option>
                  <option value="resolved">Resolved</option>
                  <option value="false_positive">False Positive</option>
                </select>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">MITRE Tactic</label>
                <select
                  value={filters.tactic ?? ''}
                  onChange={(e) => updateFilter('tactic', e.target.value || undefined)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                >
                  <option value="">All</option>
                  {Object.entries(MITRE_TACTICS).map(([id, name]) => (
                    <option key={id} value={id}>
                      {name} ({id})
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex items-end gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    setFilters({ page: 1, size: 20, range: filters.range })
                  }
                >
                  Reset Filters
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <div className="flex items-center gap-3">
            <CardTitle className="text-xs font-medium">Detection Timeline</CardTitle>
            {isHistFetching && <span className="text-[10px] text-muted-foreground">refreshing...</span>}
          </div>
          <div className="flex items-center gap-1">
            {RANGE_PRESETS.map((p) => (
              <button
                key={p.id}
                onClick={() => updateFilter('range', p.id)}
                className={`px-2 py-1 rounded-md text-xs transition-colors ${
                  filters.range === p.id
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:bg-muted'
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>
        </CardHeader>
        <CardContent>
          <ResponsiveContainer width="100%" height={160}>
            <BarChart data={histogram} margin={{ top: 4, right: 4, left: 4, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="label" tick={{ fontSize: 11 }} minTickGap={24} />
              <YAxis tick={{ fontSize: 11 }} allowDecimals={false} />
              <Tooltip
                labelFormatter={(l) => String(l)}
                formatter={(v) => [`${v} detections`, 'count']}
              />
              <Bar dataKey="count" radius={[2, 2, 0, 0]}>
                {histogram.map((h) => {
                  const ratio = maxCount ? h.count / maxCount : 0
                  const fill =
                    ratio >= 0.75 ? '#dc2626' : ratio >= 0.5 ? '#f59e0b' : '#2563eb'
                  return <Cell key={`${h.ts}-${h.count}`} fill={fill} />
                })}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-xs font-medium">All Detections</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {isLoading ? (
            <div className="flex items-center justify-center h-48">Loading...</div>
          ) : detections.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              No detections found. All clear!
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border text-left uppercase text-muted-foreground">
                    <th className="px-3 py-2">Severity</th>
                    <th className="px-3 py-2">Detection</th>
                    <th className="px-3 py-2">Rule</th>
                    <th className="px-3 py-2">Status</th>
                    <th className="px-3 py-2 text-right">Matched</th>
                    <th className="px-3 py-2 text-right">Detected</th>
                    <th className="px-3 py-2">MITRE</th>
                  </tr>
                </thead>
                <tbody>
                  {sorted.map((detection) => {
                    const rule = byId(detection.rule_id)
                    const sev = rule?.severity ?? 'info'
                    return (
                      <tr
                        key={detection.id}
                        className="border-b border-border hover:bg-muted/50 transition-colors cursor-pointer"
                        onClick={() => {
                          window.location.href = `/detections/${detection.id}`
                        }}
                      >
                        <td className="px-3 py-2">
                          <span
                            className={`inline-block min-w-14 text-center px-2 py-0.5 rounded text-[10px] font-semibold ${
                              SEVERITY_STYLES[sev] ?? ''
                            }`}
                          >
                            {sev}
                          </span>
                        </td>
                        <td className="px-3 py-2 font-mono">
                          <Link
                            to={`/detections/${detection.id}`}
                            onClick={(e) => e.stopPropagation()}
                            className="text-primary hover:underline"
                          >
                            #{detection.id.slice(0, 8)}
                          </Link>
                        </td>
                        <td className="px-3 py-2 font-mono">{rule?.name ?? detection.rule_id.slice(0, 8)}</td>
                        <td className="px-3 py-2">
                          <span
                            className={`px-2 py-0.5 rounded-full text-[10px] ${
                              STATUS_STYLES[detection.status] ?? 'bg-slate-100 text-slate-700'
                            }`}
                          >
                            {detection.status}
                          </span>
                        </td>
                        <td className="px-3 py-2 text-right font-mono">{detection.matched_count}</td>
                        <td className="px-3 py-2 text-right whitespace-nowrap">
                          {fmtDate(detection.detected_at)}
                        </td>
                        <td className="px-3 py-2">
                          <MitreBadges
                            tactics={rule?.mitre_tactics}
                            techniques={rule?.mitre_techniques}
                            compact
                          />
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      {meta && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            Showing {detections.length} of {meta.total} detections
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={filters.page <= 1}
              onClick={() => setFilters((prev) => ({ ...prev, page: prev.page - 1 }))}
            >
              Previous
            </Button>
            <span className="px-3 py-1 text-xs">
              {filters.page} / {totalPages}
            </span>
            <Button
              variant="outline"
              size="sm"
              disabled={filters.page >= totalPages}
              onClick={() => setFilters((prev) => ({ ...prev, page: prev.page + 1 }))}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
