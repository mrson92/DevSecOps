import { useQuery } from '@tanstack/react-query'
import { useParams, useNavigate } from 'react-router-dom'
import api from '@/shared/lib/api'
import type { ApiResponse, Report } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import {
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Cell,
  PieChart,
  Pie,
  Legend,
} from 'recharts'

interface ReportContent {
  period: { start: string; end: string; report_type: string }
  summary: {
    total_detections: number
    open_detections: number
    resolved_detections: number
    critical_count: number
    high_count: number
    medium_count: number
    low_count: number
    unique_rules_triggered: number
    unique_ips: number
  }
  top_rules: Array<{
    rule_id: string
    rule_name: string
    severity: string
    detection_count: number
    total_matched: number
    mitre_tactics: string[]
    recommendation: string
    evidence: string[]
  }>
  top_ips: Array<{
    ip: string
    detection_count: number
    rules_triggered: number
    evidence: string[]
  }>
  severity_breakdown: { critical: number; high: number; medium: number; low: number }
  hourly_distribution: number[]
  mitre_tactics: Array<{ tactic: string; detection_count: number }>
  recommendations: string[]
  ai_summary?: string | null
}

const SEVERITY_LABELS: Record<string, string> = {
  critical: 'Critical',
  high: 'High',
  medium: 'Medium',
  low: 'Low',
  info: 'Info',
}

const SEVERITY_COLORS: Record<string, string> = {
  critical: '#dc2626',
  high: '#f97316',
  medium: '#eab308',
  low: '#3b82f6',
  info: '#94a3b8',
}

function fmtDate(ts: string): string {
  const d = new Date(ts)
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString('ko-KR')
}

function parseContent(raw: string): ReportContent | null {
  try {
    return JSON.parse(raw) as ReportContent
  } catch {
    return null
  }
}

export function ReportDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()

  const { data: reportData, isLoading, isError } = useQuery<ApiResponse<Report>>({
    queryKey: ['report', id],
    queryFn: () => api.get(`/reports/${id}`).then((res) => res.data),
    enabled: !!id,
  })

  const report = reportData?.data
  const content = report ? parseContent(report.content) : null

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  if (isError || !report || !content) {
    return (
      <div className="space-y-4">
        <Button variant="outline" size="sm" onClick={() => navigate('/reports')}>
          ← Back to Reports
        </Button>
        <Card>
          <CardContent className="py-10 text-center text-muted-foreground">
            {report && !content ? 'Report content could not be parsed.' : 'Report not found.'}
          </CardContent>
        </Card>
      </div>
    )
  }

  const { summary, top_rules, top_ips, severity_breakdown, hourly_distribution, mitre_tactics, recommendations, ai_summary } = content

  const severityData = [
    { name: 'Critical', value: severity_breakdown.critical, color: SEVERITY_COLORS.critical },
    { name: 'High', value: severity_breakdown.high, color: SEVERITY_COLORS.high },
    { name: 'Medium', value: severity_breakdown.medium, color: SEVERITY_COLORS.medium },
    { name: 'Low', value: severity_breakdown.low, color: SEVERITY_COLORS.low },
  ].filter((d) => d.value > 0)

  const hourlyData = hourly_distribution.map((count, h) => ({
    hour: `${String(h).padStart(2, '0')}:00`,
    count,
  }))

  const topRulesData = [...top_rules]
    .sort((a, b) => b.detection_count - a.detection_count)
    .slice(0, 10)
    .map((r) => ({ name: r.rule_name, count: r.detection_count }))

  const mitreData = [...mitre_tactics].map((t) => ({
    name: t.tactic,
    count: t.detection_count,
  }))

  const summaryCards = [
    { label: 'Total Detections', value: summary.total_detections, color: 'text-foreground' },
    { label: 'Open', value: summary.open_detections, color: 'text-red-600' },
    { label: 'Resolved', value: summary.resolved_detections, color: 'text-green-600' },
    { label: 'Critical', value: summary.critical_count, color: 'text-red-700' },
    { label: 'High', value: summary.high_count, color: 'text-orange-600' },
    { label: 'Medium', value: summary.medium_count, color: 'text-yellow-600' },
    { label: 'Low', value: summary.low_count, color: 'text-blue-600' },
    { label: 'Rules Triggered', value: summary.unique_rules_triggered, color: 'text-foreground' },
    { label: 'Unique IPs', value: summary.unique_ips, color: 'text-foreground' },
  ]

  const severityBadge = (sev: string) =>
    `px-2 py-0.5 rounded text-[10px] font-semibold text-white ${
      { critical: 'bg-red-600', high: 'bg-orange-500', medium: 'bg-yellow-400 text-black', low: 'bg-blue-500', info: 'bg-slate-400' }[sev] ?? 'bg-slate-400'
    }`

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <Button variant="outline" size="sm" onClick={() => navigate('/reports')}>
            ← Back to Reports
          </Button>
          <h1 className="mt-3 text-2xl font-bold">{report.title}</h1>
          <p className="text-sm text-muted-foreground mt-1">
            {content.period.report_type} ·{' '}
            {fmtDate(content.period.start)} ~ {fmtDate(content.period.end)}
          </p>
        </div>
      </div>

      {ai_summary && (
        <Card className="border-primary/30 bg-primary/5">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">AI Executive Summary</CardTitle>
          </CardHeader>
          <CardContent className="text-sm leading-relaxed">{ai_summary}</CardContent>
        </Card>
      )}

      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
        {summaryCards.map((c) => (
          <Card key={c.label}>
            <CardContent className="py-4 text-center">
              <div className={`text-2xl font-bold ${c.color}`}>{c.value}</div>
              <div className="mt-1 text-[11px] text-muted-foreground uppercase tracking-wide">
                {c.label}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Severity Breakdown</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={280}>
              <PieChart>
                <Pie
                  data={severityData}
                  dataKey="value"
                  nameKey="name"
                  cx="50%"
                  cy="50%"
                  outerRadius={90}
                  label={(e) => `${e.name} ${e.value}`}
                >
                  {severityData.map((d) => (
                    <Cell key={d.name} fill={d.color} />
                  ))}
                </Pie>
                <Legend />
                <Tooltip formatter={(v) => [`${v}`, 'count']} />
              </PieChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Detections by Hour</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={280}>
              <BarChart data={hourlyData} margin={{ top: 4, right: 4, left: 4, bottom: 0 }}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="hour" tick={{ fontSize: 11 }} minTickGap={12} />
                <YAxis tick={{ fontSize: 11 }} allowDecimals={false} />
                <Tooltip formatter={(v) => [`${v}`, 'detections']} />
                <Bar dataKey="count" fill="#2563eb" radius={[2, 2, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Top Rules by Detection Count</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={280}>
              <BarChart
                layout="vertical"
                data={topRulesData}
                margin={{ top: 4, right: 24, left: 4, bottom: 0 }}
              >
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis type="number" tick={{ fontSize: 11 }} allowDecimals={false} />
                <YAxis
                  type="category"
                  dataKey="name"
                  width={160}
                  tick={{ fontSize: 10 }}
                />
                <Tooltip formatter={(v) => [`${v}`, 'count']} />
                <Bar dataKey="count" fill="#f97316" radius={[0, 2, 2, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">MITRE ATT&CK Tactics</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={280}>
              <BarChart
                layout="vertical"
                data={mitreData}
                margin={{ top: 4, right: 24, left: 4, bottom: 0 }}
              >
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis type="number" tick={{ fontSize: 11 }} allowDecimals={false} />
                <YAxis type="category" dataKey="name" width={120} tick={{ fontSize: 10 }} />
                <Tooltip formatter={(v) => [`${v}`, 'detections']} />
                <Bar dataKey="count" fill="#8b5cf6" radius={[0, 2, 2, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      </div>

      {top_rules.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Top Rules Detail</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border text-left uppercase text-muted-foreground">
                    <th className="px-3 py-2">Rule</th>
                    <th className="px-3 py-2">Severity</th>
                    <th className="px-3 py-2 text-right">Detections</th>
                    <th className="px-3 py-2 text-right">Matched</th>
                    <th className="px-3 py-2">MITRE Tactics</th>
                    <th className="px-3 py-2">Recommendation</th>
                  </tr>
                </thead>
                <tbody>
                  {top_rules.map((r) => (
                    <tr key={r.rule_id} className="border-b border-border/60">
                      <td className="px-3 py-2 font-mono">{r.rule_name}</td>
                      <td className="px-3 py-2">
                        <span className={severityBadge(r.severity)}>
                          {SEVERITY_LABELS[r.severity] ?? r.severity}
                        </span>
                      </td>
                      <td className="px-3 py-2 text-right font-mono">{r.detection_count}</td>
                      <td className="px-3 py-2 text-right font-mono">{r.total_matched}</td>
                      <td className="px-3 py-2">
                        <div className="flex flex-wrap gap-1">
                          {r.mitre_tactics.length > 0
                            ? r.mitre_tactics.map((t) => (
                                <span key={t} className="px-1.5 py-0.5 rounded bg-violet-100 text-violet-700 text-[10px]">
                                  {t}
                                </span>
                              ))
                            : '-'}
                        </div>
                      </td>
                      <td className="px-3 py-2 max-w-xs text-muted-foreground">{r.recommendation || '-'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}

      {top_ips.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Top Source IPs</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border text-left uppercase text-muted-foreground">
                    <th className="px-3 py-2">IP Address</th>
                    <th className="px-3 py-2 text-right">Detections</th>
                    <th className="px-3 py-2 text-right">Rules Triggered</th>
                  </tr>
                </thead>
                <tbody>
                  {top_ips.map((ip) => (
                    <tr key={ip.ip} className="border-b border-border/60">
                      <td className="px-3 py-2 font-mono">{ip.ip}</td>
                      <td className="px-3 py-2 text-right font-mono">{ip.detection_count}</td>
                      <td className="px-3 py-2 text-right font-mono">{ip.rules_triggered}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}

      {recommendations.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Recommendations (조치 방안)</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {recommendations.map((rec, i) => (
              <div key={i} className="flex gap-3 items-start rounded-lg border border-border p-3">
                <span className="shrink-0 w-6 h-6 rounded-full bg-primary/10 text-primary flex items-center justify-center text-xs font-bold">
                  {i + 1}
                </span>
                <div className="text-sm leading-relaxed">{rec}</div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {report.summary && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Raw Summary</CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="p-3 bg-muted rounded-md text-xs overflow-x-auto whitespace-pre-wrap">
              {report.summary}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
