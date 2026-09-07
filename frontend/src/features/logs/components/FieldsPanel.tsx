import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { DatasourceField, FieldValuesResponse } from '@/shared/types'
import { Card } from '@/components/ui/card'

const COLLAPSED_KEY = 'aads-logs-fields-collapsed'

export const FIELD_KEYS = ['source', 'client_ip', 'method', 'path', 'status_code', 'user_id', 'user_agent'] as const
export type FieldKey = (typeof FIELD_KEYS)[number]

export interface FieldSpec {
  label: string
  key: FieldKey
  aliases: string[]
  logKey: string
}

const FIELD_SPECS: FieldSpec[] = [
  { label: 'Source', key: 'source', aliases: ['source'], logKey: 'source' },
  { label: 'Client IP', key: 'client_ip', aliases: ['client_ip', 'network.client.ip'], logKey: 'client_ip' },
  { label: 'Method', key: 'method', aliases: ['method', 'http.request.method'], logKey: 'method' },
  { label: 'Path', key: 'path', aliases: ['path', 'http.request.path'], logKey: 'path' },
  { label: 'Status', key: 'status_code', aliases: ['status_code', 'http.response.status_code'], logKey: 'status_code' },
  { label: 'User ID', key: 'user_id', aliases: ['user_id', 'app.user.id'], logKey: 'user_id' },
  { label: 'User Agent', key: 'user_agent', aliases: ['user_agent', 'http.user_agent.original'], logKey: 'user_agent' },
]

export interface ActiveFilter {
  key: FieldKey
  value: string
  negate: boolean
}

export function matchSpec(field: DatasourceField): FieldSpec | undefined {
  return FIELD_SPECS.find((s) => {
    const keys = [field.name, field.standard ?? ''] as const
    return keys.some((k) => {
      const base = k.split('.').pop() ?? k
      return s.aliases.includes(k) || s.aliases.includes(base)
    })
  })
}

function shortType(t: string): string {
  const match = t.match(/^[A-Za-z]+/)
  return match ? match[0] : t
}

function isAggregatable(t: string): boolean {
  const base = shortType(t)
  if (/^(DateTime|Date|UInt|Int|Float|Bool)$/i.test(base)) return true
  if (/^(keyword|long|integer|short|byte|double|float|boolean|date)$/i.test(base)) return true
  return /string$/i.test(base)
}

function loadCollapsed(): boolean {
  try {
    return localStorage.getItem(COLLAPSED_KEY) === '1'
  } catch {
    return false
  }
}

interface FieldsPanelProps {
  datasource: string
  from?: string
  to?: string
  fields: DatasourceField[]
  isFieldsLoading: boolean
  activeFilters: ActiveFilter[]
  onAddFilter: (spec: FieldSpec, value: string, negate: boolean) => void
}

function isActive(filters: ActiveFilter[], spec: FieldSpec | undefined, value: string) {
  if (!spec) return { included: false, excluded: false }
  const included = filters.some((f) => f.key === spec.key && !f.negate && f.value === value)
  const excluded = filters.some((f) => f.key === spec.key && f.negate && f.value === value)
  return { included, excluded }
}

export function FieldsPanel({
  datasource,
  from,
  to,
  fields,
  isFieldsLoading,
  activeFilters,
  onAddFilter,
}: FieldsPanelProps) {
  const [collapsed, setCollapsed] = useState(loadCollapsed)
  const [selected, setSelected] = useState<DatasourceField | null>(null)
  const [fieldQ, setFieldQ] = useState('')
  const [valueInput, setValueInput] = useState('')
  const [valueQ, setValueQ] = useState('')

  useEffect(() => {
    const id = window.setTimeout(() => setValueQ(valueInput.trim()), 300)
    return () => window.clearTimeout(id)
  }, [valueInput])

  const fq = fieldQ.trim().toLowerCase()
  const visibleFields = fields.filter((f) => {
    const name = f.name.toLowerCase()
    const std = (f.standard ?? '').toLowerCase()
    return name.includes(fq) || std.includes(fq) || shortType(f.type).toLowerCase().includes(fq)
  })

  const toggle = () => {
    setCollapsed((prev) => {
      try {
        localStorage.setItem(COLLAPSED_KEY, prev ? '0' : '1')
      } catch {
        /* ignore */
      }
      return !prev
    })
  }

  if (collapsed) {
    return (
      <Card className="w-9 shrink-0 flex flex-col items-center py-2">
        <button
          onClick={toggle}
          title="Expand fields"
          className="p-1 rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
        >
          <span aria-hidden>»</span>
        </button>
        <span className="mt-2 text-[10px] text-muted-foreground [writing-mode:vertical-rl] rotate-180">
          Fields
        </span>
      </Card>
    )
  }

  const { data: valuesData, isFetching: isValuesFetching } = useQuery<FieldValuesResponse>({
    queryKey: ['logs-field-values', datasource, selected?.name, valueQ, from, to],
    enabled: !!selected,
    queryFn: () => {
      const params = new URLSearchParams()
      params.set('field', selected!.name)
      params.set('size', '10')
      if (datasource) params.set('datasource', datasource)
      if (from) params.set('from', from)
      if (to) params.set('to', to)
      if (valueQ) params.set('q', valueQ)
      return api.get(`/logs/fields/values?${params.toString()}`).then((res) => res.data)
    },
  })

  const values = valuesData?.data ?? []
  const maxCount = values.length > 0 ? Math.max(...values.map((v) => v.count)) : 0
  const spec = selected ? matchSpec(selected) : undefined

  const openField = (f: DatasourceField) => {
    setSelected(f)
    setValueInput('')
    setValueQ('')
  }

  return (
    <Card className="w-80 shrink-0 flex flex-col">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-border shrink-0">
        <h2 className="text-xs font-semibold uppercase">Fields</h2>
        <span className="text-[10px] text-muted-foreground">
          {isFieldsLoading ? '…' : fields.length}
        </span>
        <button
          onClick={toggle}
          title="Collapse fields"
          className="ml-auto p-1 rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
        >
          <span aria-hidden>«</span>
        </button>
      </div>

      {selected ? (
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="p-2 border-b border-border space-y-1 shrink-0">
            <div className="flex items-center gap-1">
              <button
                onClick={() => setSelected(null)}
                title="Back to all fields"
                className="p-1 rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
              >
                <span aria-hidden>←</span>
              </button>
              <span className="text-[11px] font-mono font-semibold truncate" title={selected.name}>
                {selected.name}
              </span>
              <span className="text-[9px] text-muted-foreground truncate">{shortType(selected.type)}</span>
            </div>
            <input
              type="text"
              value={valueInput}
              onChange={(e) => setValueInput(e.target.value)}
              placeholder="Search values..."
              className="w-full px-2 py-1.5 rounded-md border border-border bg-background text-xs"
            />
          </div>
          <div className="px-2 py-2 flex-1 overflow-y-auto space-y-1">
            {isValuesFetching && values.length === 0 ? (
              <div className="text-[10px] text-muted-foreground px-1 py-2">Loading values…</div>
            ) : !isAggregatable(selected.type) ? (
              <div className="text-[10px] text-muted-foreground px-1 py-2">
                Field of type “{selected.type}” cannot be aggregated.
              </div>
            ) : values.length === 0 ? (
              <div className="text-[10px] text-muted-foreground px-1 py-2">
                No values in the current time window{valueQ ? ` for “${valueQ}”` : ''}.
              </div>
            ) : (
              values.map((v) => {
                const active = isActive(activeFilters, spec, v.value)
                return (
                  <div
                    key={`${selected.name}:${v.value}`}
                    className="group w-full text-left rounded px-1.5 py-1 hover:bg-muted/60 transition-colors"
                  >
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => spec && onAddFilter(spec, v.value, false)}
                        title={spec ? `Filter by ${spec.label} = ${v.value}` : `Search "${v.value}"`}
                        className="flex-1 min-w-0 flex items-center gap-1"
                      >
                        <span
                          className={`text-[10px] font-mono truncate ${
                            active.included ? 'text-primary font-semibold' : ''
                          }`}
                        >
                          {v.value}
                        </span>
                        <span className="text-[9px] text-muted-foreground shrink-0">{v.count}</span>
                      </button>
                      {spec && (
                        <span className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                          <button
                            onClick={() => onAddFilter(spec, v.value, false)}
                            title={`Include ${spec.label} = ${v.value}`}
                            className={`w-4 h-4 rounded text-[10px] leading-none flex items-center justify-center transition-colors ${
                              active.included
                                ? 'bg-primary text-primary-foreground'
                                : 'text-muted-foreground hover:bg-primary/15 hover:text-primary'
                            }`}
                          >
                            +
                          </button>
                          <button
                            onClick={() => onAddFilter(spec, v.value, true)}
                            title={`Exclude ${spec.label} = ${v.value}`}
                            className={`w-4 h-4 rounded text-[10px] leading-none flex items-center justify-center transition-colors ${
                              active.excluded
                                ? 'bg-destructive text-destructive-foreground'
                                : 'text-muted-foreground hover:bg-destructive/15 hover:text-destructive'
                            }`}
                          >
                            −
                          </button>
                        </span>
                      )}
                    </div>
                    <span className="block h-[3px] rounded-full bg-primary/25 mt-0.5">
                      <span
                        className="block h-full rounded-full bg-primary/60"
                        style={{ width: `${maxCount ? (v.count / maxCount) * 100 : 0}%` }}
                      />
                    </span>
                  </div>
                )
              })
            )}
          </div>
          <div className="px-3 py-1.5 border-t border-border text-[10px] text-muted-foreground shrink-0 space-y-0.5">
            <div>+ Click to add (include)</div>
            <div>− Click to exclude</div>
          </div>
        </div>
      ) : (
        <>
          <div className="p-2 border-b border-border shrink-0">
            <input
              type="text"
              value={fieldQ}
              onChange={(e) => setFieldQ(e.target.value)}
              placeholder="Search fields..."
              className="w-full px-2 py-1.5 rounded-md border border-border bg-background text-xs"
            />
            <div className="text-[10px] text-muted-foreground px-1 pt-1">
              {isFieldsLoading
                ? 'Loading fields…'
                : fields.length === 0
                  ? 'No fields found for this source.'
                  : `${visibleFields.length}${fq ? '/' + fields.length : ''} field${fields.length === 1 ? '' : 's'}`}
            </div>
          </div>
          <div className="px-2 py-1.5 overflow-y-auto max-h-[58vh] shrink-0 space-y-0.5">
            {visibleFields.map((f) => {
              const isSpec = matchSpec(f)
              return (
                <button
                  key={f.name}
                  onClick={() => openField(f)}
                  title="Open field"
                  className="w-full flex items-center gap-1.5 px-1.5 py-[3px] rounded text-left hover:bg-muted/60 transition-colors"
                >
                  <span className="text-[11px] font-mono truncate flex-1" title={f.name}>
                    {f.name}
                  </span>
                  <span className="text-[9px] text-muted-foreground shrink-0">{shortType(f.type)}</span>
                  {isSpec && <span className="text-[9px] text-emerald-600 shrink-0">F</span>}
                </button>
              )
            })}
            {visibleFields.length === 0 && (
              <div className="py-4 text-center text-[11px] text-muted-foreground">
                No matching fields.
              </div>
            )}
          </div>
        </>
      )}
    </Card>
  )
}
