import { Fragment, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type {
  ApiResponse,
  DataSource,
  DatasourceFieldResponse,
  LogEntry,
  LogHistogramBucket,
  LogSearchResponse,
} from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts'
import { FieldsPanel, type ActiveFilter, type FieldSpec } from '../components/FieldsPanel'

const RANGE_PRESETS = [
  { id: '1h', label: 'Last 1 hour', hours: 1 },
  { id: '6h', label: 'Last 6 hours', hours: 6 },
  { id: '24h', label: 'Last 24 hours', hours: 24 },
  { id: '7d', label: 'Last 7 days', hours: 168 },
] as const

const DATA_SOURCE_KEY = 'aads-logs-datasource'

const DEFAULT_COLUMNS = ['timestamp', 'method', 'source', 'path', 'status_code', 'response_size', 'response_time', 'user_id'] as const

type ColumnKey =
  | 'timestamp'
  | 'method'
  | 'source'
  | 'path'
  | 'status_code'
  | 'client_ip'
  | 'user_id'
  | 'user_agent'
  | 'query'
  | 'response_size'
  | 'response_time'

const FILTER_LABELS: Record<string, string> = {
  source: 'Source',
  client_ip: 'Client IP',
  method: 'Method',
  path: 'Path',
  status_code: 'Status',
  user_id: 'User ID',
  user_agent: 'User Agent',
}

function loadDataSource(): string {
  try {
    return localStorage.getItem(DATA_SOURCE_KEY) ?? ''
  } catch {
    return ''
  }
}

function saveDataSource(id: string) {
  try {
    localStorage.setItem(DATA_SOURCE_KEY, id)
  } catch {
    /* ignore */
  }
}

interface DraftFilters {
  preset: string
  customFrom: string
  customTo: string
  datasource: string
  q: string
  order: 'desc' | 'asc'
}

interface AppliedFilters {
  from?: string
  to?: string
  datasource?: string
  q?: string
  order: 'desc' | 'asc'
  offset: number
  limit: number
  filters: ActiveFilter[]
}

function pickInterval(fromMs: number, toMs: number): number {
  const durMs = Math.max(0, toMs - fromMs)
  if (durMs <= 3600_000) return 60
  if (durMs <= 6 * 3600_000) return 300
  if (durMs <= 24 * 3600_000) return 900
  return 3600
}

function resolveRange(draft: DraftFilters): { from?: string; to?: string } {
  if (draft.preset === 'custom') {
    const from = draft.customFrom ? new Date(draft.customFrom).toISOString() : undefined
    const to = draft.customTo ? new Date(draft.customTo).toISOString() : undefined
    return { from, to }
  }
  const preset = RANGE_PRESETS.find((p) => p.id === draft.preset)
  const to = new Date()
  const from = new Date(to.getTime() - (preset?.hours ?? 1) * 3600_000)
  return { from: from.toISOString(), to: to.toISOString() }
}

function parseLogTs(ts: string): Date | null {
  if (!ts) return null
  const normalized = ts.includes('T') ? ts : ts.replace(' ', 'T')
  const d = new Date(normalized)
  return Number.isNaN(d.getTime()) ? null : d
}

const statusClass = (code: number) =>
  code >= 500 ? 'bg-red-100 text-red-800'
    : code >= 400 ? 'bg-orange-100 text-orange-800'
    : code >= 300 ? 'bg-yellow-100 text-yellow-800'
    : 'bg-green-100 text-green-800'

function fmtDate(ts: string, tz: 'local' | 'utc'): string {
  const d = parseLogTs(ts)
  if (!d) return ts
  if (tz === 'utc') {
    return d.toISOString().slice(0, 19).replace('T', ' ')
  }
  return d.toLocaleString('ko-KR')
}

export function LogsPage() {
  const [draft, setDraft] = useState<DraftFilters>({
    preset: '1h',
    customFrom: '',
    customTo: '',
    datasource: loadDataSource(),
    q: '',
    order: 'desc',
  })
  const [applied, setApplied] = useState<AppliedFilters>({
    ...resolveRange(draft),
    datasource: loadDataSource() || undefined,
    order: 'desc',
    offset: 0,
    limit: 100,
    filters: [],
  })
  const [selected, setSelected] = useState<LogEntry | null>(null)
  const [tz, setTz] = useState<'local' | 'utc'>('local')
  const [pageSize, setPageSize] = useState(100)
  const [columns, setColumns] = useState<ColumnKey[]>([...DEFAULT_COLUMNS])
  const [showColumnPicker, setShowColumnPicker] = useState(false)
  const [brushRange, setBrushRange] = useState<{ fromTs: number | null; toTs: number | null }>({
    fromTs: null,
    toTs: null,
  })

  const { data: dataSources } = useQuery<ApiResponse<DataSource[]>>({
    queryKey: ['data-sources'],
    queryFn: () => api.get('/data-sources').then((res) => res.data),
  })
  const logSources = (dataSources?.data ?? []).filter(
    (s) => s.enabled && (s.type === 'clickhouse' || s.type === 'elasticsearch')
  )

  const datasourceKey = applied.datasource ?? ''
  const { data: fieldsData, isLoading: isFieldsLoading } = useQuery<DatasourceFieldResponse>({
    queryKey: ['logs-fields', datasourceKey],
    queryFn: () =>
      api
        .get(`/logs/fields${datasourceKey ? `?datasource=${encodeURIComponent(datasourceKey)}` : ''}`)
        .then((res) => res.data),
  })

  const queryParams = useMemo(() => {
    const params = new URLSearchParams()
    params.set('limit', String(applied.limit))
    params.set('offset', String(applied.offset))
    params.set('order', applied.order)
    if (applied.from) params.set('from', applied.from)
    if (applied.to) params.set('to', applied.to)
    if (applied.datasource) params.set('datasource', applied.datasource)
    if (applied.q) params.set('q', applied.q)
    for (const f of applied.filters) {
      const key: string = f.key
      const exact: Record<string, string> = {
        source: 'source',
        client_ip: 'client_ip',
        method: 'method',
        path: 'path',
        user_id: 'user_id',
        user_agent: 'user_agent',
      }
      if (key === 'status_code') {
        params.set('status_code', f.value)
      } else if (exact[key]) {
        params.set(exact[key], f.value)
      }
      if (f.negate) {
        if (key === 'status_code') {
          params.set('exclude_status_code', f.value)
        } else if (exact[key]) {
          params.set(`exclude_${exact[key]}`, f.value)
        }
      }
    }
    return params
  }, [applied])

  const interval = useMemo(
    () => pickInterval(Date.parse(applied.from ?? ''), Date.parse(applied.to ?? '')),
    [applied]
  )

  const { data: searchData, isLoading, isFetching } = useQuery<LogSearchResponse>({
    queryKey: ['logs-search', queryParams.toString()],
    queryFn: () => api.get(`/logs?${queryParams.toString()}`).then((res) => res.data),
  })

  const { data: histData } = useQuery<ApiResponse<LogHistogramBucket[]>>({
    queryKey: ['logs-histogram', queryParams.toString(), interval],
    queryFn: () => {
      const params = new URLSearchParams(queryParams)
      params.set('interval', String(interval))
      params.delete('limit')
      params.delete('offset')
      return api.get(`/logs/histogram?${params.toString()}`).then((res) => res.data)
    },
  })

  const logs = searchData?.data ?? []
  const meta = searchData?.meta
  const histBuckets = histData?.data ?? []
  const histogram = histBuckets.map((b) => ({
    ts: b.ts,
    label: new Date(b.ts * 1000).toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' }),
    count: b.count,
  }))

  const setD = (key: keyof DraftFilters, value: string) =>
    setDraft((prev) => ({ ...prev, [key]: value }))

  const buildApplied = (over: Partial<AppliedFilters> = {}): AppliedFilters => {
    const range = resolveRange(draft)
    return {
      ...range,
      datasource: draft.datasource || undefined,
      q: draft.q || undefined,
      order: draft.order,
      offset: 0,
      limit: pageSize,
      filters: applied.filters,
      ...over,
    }
  }

  const apply = () => {
    setApplied(buildApplied())
    setSelected(null)
    setBrushRange({ fromTs: null, toTs: null })
  }

  const reset = () => {
    setDraft((prev) => ({
      ...prev,
      q: '',
      preset: '1h',
      order: 'desc',
    }))
    setApplied({
      ...resolveRange({ ...draft, preset: '1h', q: '' }),
      datasource: draft.datasource || undefined,
      order: 'desc',
      offset: 0,
      limit: pageSize,
      filters: [],
    })
    setSelected(null)
    setBrushRange({ fromTs: null, toTs: null })
  }

  const applyDatasource = (id: string) => {
    saveDataSource(id)
    setDraft((prev) => ({ ...prev, datasource: id }))
    setApplied((prev) => ({ ...prev, datasource: id || undefined, offset: 0, filters: prev.filters }))
    setSelected(null)
  }

  const addFilter = (spec: FieldSpec, value: string, negate: boolean) => {
    setApplied((prev) => {
      const rest = prev.filters.filter(
        (f) => !(f.key === spec.key && f.value === value)
      )
      return { ...prev, filters: [...rest, { key: spec.key, value, negate }], offset: 0 }
    })
    setSelected(null)
  }

  const removeFilter = (index: number) => {
    setApplied((prev) => {
      const filters = prev.filters.filter((_, i) => i !== index)
      return { ...prev, filters, offset: 0 }
    })
    setSelected(null)
  }

  const toggleNegate = (index: number) => {
    setApplied((prev) => {
      const filters = prev.filters.map((f, i) => (i === index ? { ...f, negate: !f.negate } : f))
      return { ...prev, filters, offset: 0 }
    })
    setSelected(null)
  }

  const changePageSize = (size: number) => {
    setPageSize(size)
    setApplied((prev) => ({ ...prev, limit: size, offset: 0 }))
  }

  const changeTz = (v: 'local' | 'utc') => {
    setTz(v)
    setSelected(null)
  }

  const toggleColumn = (col: ColumnKey) => {
    setColumns((prev) => (prev.includes(col) ? prev.filter((c) => c !== col) : [...prev, col]))
  }

  const allColumns: ColumnKey[] = [
    'timestamp', 'method', 'source', 'path', 'status_code',
    'client_ip', 'user_id', 'user_agent', 'query', 'response_size', 'response_time',
  ]

  const handleBrush = (index: number) => {
    const bucket = histogram[index]
    if (!bucket) return
    const current = brushRange
    if (current.fromTs == null) {
      setBrushRange({ fromTs: bucket.ts, toTs: null })
    } else if (current.fromTs != null && current.toTs == null) {
      const fromMs = Math.min(current.fromTs, bucket.ts) * 1000
      const toMs = Math.max(current.fromTs, bucket.ts) * 1000 + interval * 1000 - 1
      applyBrush(fromMs, toMs)
      setBrushRange({ fromTs: null, toTs: null })
    }
  }

  const applyBrush = (fromMs: number, toMs: number) => {
    setDraft((prev) => ({
      ...prev,
      preset: 'custom',
      customFrom: '',
      customTo: '',
    }))
    const fromIso = new Date(fromMs).toISOString()
    const toIso = new Date(toMs).toISOString()
    setApplied((prev) => ({
      ...prev,
      from: fromIso,
      to: toIso,
      offset: 0,
    }))
  }

  const firstOffset = applied.offset - applied.limit
  const hasPrev = firstOffset >= 0
  const hasNext = meta?.has_more ?? false
  const total = meta?.total ?? logs.length
  const shownLabel = logs.length === 0
    ? '0'
    : `${firstOffset + 1}-${firstOffset + logs.length}`

  const inputClass =
    'w-full px-3 py-1.5 rounded-md border border-border bg-background text-xs font-mono'

  const selectClass =
    'w-full px-2 py-1.5 rounded-md border border-border bg-background text-xs'

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <h1 className="text-2xl font-bold">Logs</h1>
        <div className="flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground whitespace-nowrap">Data Source</span>
            <select
              value={draft.datasource}
              onChange={(e) => applyDatasource(e.target.value)}
              className="px-3 py-2 rounded-md border border-border bg-background text-sm min-w-44"
            >
              <option value="">Auto (primary)</option>
              {logSources.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name} ({s.type}){s.is_primary && ' ★'}
                </option>
              ))}
            </select>
          </div>
          {meta && (
            <span className="text-xs text-muted-foreground">
              → {meta.datasource_name} ({meta.datasource_type})
            </span>
          )}
        </div>
      </div>

      <div className="flex gap-4 items-start">
        <FieldsPanel
          datasource={datasourceKey}
          from={applied.from}
          to={applied.to}
          fields={fieldsData?.data ?? []}
          isFieldsLoading={isFieldsLoading}
          activeFilters={applied.filters}
          onAddFilter={addFilter}
        />

        <div className="flex-1 min-w-0 space-y-4">
      <Card>
        <CardContent className="py-4 space-y-4">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="space-y-2">
              <label className="text-xs font-medium">Time Range</label>
              <select
                value={draft.preset}
                onChange={(e) => setD('preset', e.target.value)}
                className={selectClass}
              >
                {RANGE_PRESETS.map((p) => (
                  <option key={p.id} value={p.id}>{p.label}</option>
                ))}
                <option value="custom">Custom</option>
              </select>
            </div>
            <div className="space-y-2">
              <label className="text-xs font-medium">Search</label>
              <input
                type="text"
                value={draft.q}
                onChange={(e) => setD('q', e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && apply()}
                placeholder="path, URL, IP, UA..."
                className={inputClass}
              />
            </div>
            <div className="flex items-end gap-2">
              <Button onClick={apply} className="flex-1">Search</Button>
              <Button variant="outline" onClick={reset}>Reset</Button>
            </div>
          </div>

          {draft.preset === 'custom' && (
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="text-xs font-medium">From</label>
                <input
                  type="datetime-local"
                  value={draft.customFrom}
                  onChange={(e) => setD('customFrom', e.target.value)}
                  className={selectClass}
                />
              </div>
              <div className="space-y-2">
                <label className="text-xs font-medium">To</label>
                <input
                  type="datetime-local"
                  value={draft.customTo}
                  onChange={(e) => setD('customTo', e.target.value)}
                  className={selectClass}
                />
              </div>
            </div>
          )}

          <div className="flex items-center gap-3">
            <label className="text-xs font-medium">Order</label>
            <select
              value={draft.order}
              onChange={(e) => setD('order', e.target.value)}
              className={selectClass + ' w-auto'}
            >
              <option value="desc">Newest first</option>
              <option value="asc">Oldest first</option>
            </select>
            <label className="text-xs font-medium ml-2">Timezone</label>
            <select
              value={tz}
              onChange={(e) => changeTz(e.target.value as 'local' | 'utc')}
              className={selectClass + ' w-auto'}
            >
              <option value="local">Local</option>
              <option value="utc">UTC</option>
            </select>
          </div>
        </CardContent>
      </Card>

      {applied.filters.length > 0 && (
        <Card>
          <CardContent className="py-3 flex flex-wrap items-center gap-2">
            <span className="text-xs text-muted-foreground">Filters:</span>
            {applied.filters.map((f, i) => {
              return (
                <span
                  key={`${f.key}:${f.value}:${i}`}
                  className={`inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs border ${
                    f.negate
                      ? 'bg-destructive/10 border-destructive/30 text-destructive'
                      : 'bg-primary/10 border-primary/30 text-primary'
                  }`}
                >
                  <button
                    onClick={() => toggleNegate(i)}
                    title={f.negate ? 'Click to include' : 'Click to exclude'}
                    className="font-bold"
                  >
                    {f.negate ? 'NOT' : ''}
                  </button>
                  <span className="font-mono">{FILTER_LABELS[f.key] ?? f.key}</span>
                  <span className="font-mono font-semibold">= {f.value}</span>
                  <button
                    onClick={() => removeFilter(i)}
                    title="Remove filter"
                    className="hover:opacity-70"
                  >
                    ×
                  </button>
                </span>
              )
            })}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <div className="flex items-center gap-2">
            <CardTitle className="text-xs font-medium">Timeline</CardTitle>
            <span className="text-[10px] text-muted-foreground">
              {interval}s buckets · click/drag to select range
            </span>
          </div>
          {isFetching && <span className="text-xs text-muted-foreground">refreshing...</span>}
        </CardHeader>
        <CardContent>
          <ResponsiveContainer width="100%" height={140}>
            <BarChart
              data={histogram}
              margin={{ top: 4, right: 4, left: 4, bottom: 0 }}
            >
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="label" tick={{ fontSize: 12 }} />
              <YAxis tick={{ fontSize: 12 }} allowDecimals={false} />
              <Tooltip
                labelFormatter={(l) => String(l)}
                formatter={(v) => [`${v} logs`, 'count']}
              />
              <Bar dataKey="count" radius={[2, 2, 0, 0]}>
                {histogram.map((h, idx) => {
                  const isSelectedStart = brushRange.fromTs != null && h.ts === brushRange.fromTs
                  return (
                    <Cell
                      key={h.ts}
                      fill={isSelectedStart ? '#2563eb' : '#3b82f6'}
                      cursor="pointer"
                      onClick={() => handleBrush(idx)}
                    />
                  )
                })}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-xs font-medium">Log Entries</CardTitle>
          <div className="flex items-center gap-3">
            <span className="text-xs text-muted-foreground">
              {isLoading ? 'Loading...' : `Showing ${shownLabel} of ${total}`}
            </span>
            <select
              value={pageSize}
              onChange={(e) => changePageSize(Number(e.target.value))}
              className={selectClass + ' w-auto'}
              title="Page size"
            >
              {[25, 50, 100, 200, 500].map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
            <div className="relative">
              <Button variant="outline" size="sm" onClick={() => setShowColumnPicker((v) => !v)}>
                Columns ({columns.length})
              </Button>
              {showColumnPicker && (
                <div className="absolute right-0 top-full mt-1 z-20 w-48 rounded-md border border-border bg-background shadow-lg p-1">
                  {allColumns.map((col) => (
                    <label
                      key={col}
                      className="flex items-center gap-2 px-2 py-1.5 rounded text-xs hover:bg-muted cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={columns.includes(col)}
                        onChange={() => toggleColumn(col)}
                      />
                      <span className="font-mono">{col}</span>
                    </label>
                  ))}
                </div>
              )}
            </div>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          {isLoading ? (
            <div className="flex items-center justify-center h-48">Loading...</div>
          ) : logs.length === 0 ? (
            <div className="py-10 text-center text-muted-foreground">No logs found.</div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border text-left text-xs uppercase text-muted-foreground">
                    <th className="px-3 py-1.5 w-6"></th>
                    {columns.includes('timestamp') && <th className="px-3 py-1.5">Time</th>}
                    {columns.includes('source') && <th className="px-3 py-1.5">Source</th>}
                    {columns.includes('method') && <th className="px-3 py-1.5">Method</th>}
                    {columns.includes('path') && <th className="px-3 py-1.5">Path</th>}
                    {columns.includes('status_code') && <th className="px-3 py-1.5">Status</th>}
                    {columns.includes('client_ip') && <th className="px-3 py-1.5">Client IP</th>}
                    {columns.includes('user_id') && <th className="px-3 py-1.5">User</th>}
                    {columns.includes('user_agent') && <th className="px-3 py-1.5">User Agent</th>}
                    {columns.includes('query') && <th className="px-3 py-1.5">Query</th>}
                    {columns.includes('response_size') && <th className="px-3 py-1.5 text-right">Size</th>}
                    {columns.includes('response_time') && <th className="px-3 py-1.5 text-right">Response (ms)</th>}
                  </tr>
                </thead>
                <tbody>
                  {logs.map((log, i) => (
                    <Fragment key={`${log.timestamp}-${i}`}>
                      <tr
                        onClick={() => setSelected(selected === log ? null : log)}
                        className={`
                          border-b border-border cursor-pointer hover:bg-muted/50
                          ${selected === log ? 'bg-muted/60' : ''}
                        `}
                      >
                        <td className="px-3 py-1.5">
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              setSelected(selected === log ? null : log)
                            }}
                            className="text-muted-foreground hover:text-foreground"
                            title="Expand details"
                          >
                            {selected === log ? '▾' : '▸'}
                          </button>
                        </td>
                        {columns.includes('timestamp') && (
                          <td className="px-3 py-1.5 whitespace-nowrap font-mono text-xs">
                            {fmtDate(log.timestamp, tz)}
                          </td>
                        )}
                        {columns.includes('source') && (
                          <td className="px-3 py-1.5">
                            <span className="px-1.5 py-0.5 rounded bg-slate-100 text-slate-700 text-[10px]">
                              {log.source || '-'}
                            </span>
                          </td>
                        )}
                        {columns.includes('method') && (
                          <td className="px-3 py-1.5">
                            <span className="px-1.5 py-0.5 rounded bg-blue-50 text-blue-700 font-mono text-[10px]">
                              {log.method || '-'}
                            </span>
                          </td>
                        )}
                        {columns.includes('path') && (
                          <td className="px-3 py-1.5 font-mono text-xs max-w-72 truncate">{log.path || '-'}</td>
                        )}
                        {columns.includes('status_code') && (
                          <td className="px-3 py-1.5">
                            <span className={`px-2 py-0.5 rounded-full text-xs ${statusClass(log.status_code)}`}>
                              {log.status_code || '-'}
                            </span>
                          </td>
                        )}
                        {columns.includes('client_ip') && (
                          <td className="px-3 py-1.5 font-mono text-xs">{log.client_ip || '-'}</td>
                        )}
                        {columns.includes('user_id') && (
                          <td className="px-3 py-1.5 font-mono text-xs">{log.user_id || '-'}</td>
                        )}
                        {columns.includes('user_agent') && (
                          <td className="px-3 py-1.5 font-mono text-xs max-w-40 truncate">{log.user_agent || '-'}</td>
                        )}
                        {columns.includes('query') && (
                          <td className="px-3 py-1.5 font-mono text-xs max-w-40 truncate">{log.query || '-'}</td>
                        )}
                        {columns.includes('response_size') && (
                          <td className="px-3 py-1.5 text-right font-mono text-xs">{log.response_size || '-'}</td>
                        )}
                        {columns.includes('response_time') && (
                          <td className="px-3 py-1.5 text-right font-mono text-xs">
                            {log.response_time == null ? '-' : Math.round(log.response_time)}
                          </td>
                        )}
                      </tr>
                      {selected === log && (
                        <tr className="border-b border-border bg-slate-50">
                          <td colSpan={columns.length + 1} className="px-3 py-3">
                            <div className="grid md:grid-cols-2 gap-x-8 gap-y-1 text-xs">
                              {[
                                ['timestamp', fmtDate(log.timestamp, tz)],
                                ['source', log.source],
                                ['client_ip', log.client_ip],
                                ['method', log.method],
                                ['path', log.path],
                                ['query', log.query],
                                ['status_code', String(log.status_code)],
                                ['response_size', log.response_size != null ? String(log.response_size) : null],
                                ['user_agent', log.user_agent],
                                ['user_id', log.user_id],
                                ['response_time', log.response_time != null ? `${Math.round(log.response_time)} ms` : null],
                              ]
                                .filter(([, v]) => v != null && v !== '')
                                .map(([k, v]) => (
                                  <div key={k} className="flex gap-2 border-b border-border/40 py-0.5">
                                    <span className="font-mono text-muted-foreground w-32 shrink-0">{k}</span>
                                    <span className="font-mono break-all">{v}</span>
                                  </div>
                                ))}
                            </div>
                            {log.extra && Object.keys(log.extra).length > 0 && (
                              <details className="mt-2">
                                <summary className="text-[10px] text-muted-foreground cursor-pointer">
                                  Raw JSON ({Object.keys(log.extra).length} extra fields)
                                </summary>
                                <pre className="mt-1 bg-slate-950 text-slate-100 rounded-md p-2 overflow-x-auto text-xs leading-relaxed">
                                  {JSON.stringify({ ...log, extra: log.extra }, null, 2)}
                                </pre>
                              </details>
                            )}
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      {(logs.length > 0 || hasPrev || hasNext) && !isLoading && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            {meta ? `${meta.datasource_name} · ${total.toLocaleString()} total` : ''}
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={!hasPrev}
              onClick={() => setApplied((prev) => ({ ...prev, offset: prev.offset - prev.limit }))}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={!hasNext}
              onClick={() => setApplied((prev) => ({ ...prev, offset: prev.offset + prev.limit }))}
            >
              Next
            </Button>
          </div>
        </div>
      )}
      </div>
      </div>
    </div>
  )
}
