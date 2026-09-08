import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import api from '@/shared/lib/api'
import type { Report, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

type ReportCategory = 'all' | 'daily' | 'weekly' | 'monthly'

const CATEGORY_LABELS: Record<string, string> = {
  all: 'All',
  daily: 'Daily',
  weekly: 'Weekly',
  monthly: 'Monthly',
}

const CATEGORY_ORDER: ReportCategory[] = ['all', 'daily', 'weekly', 'monthly']

const STATUS_STYLES: Record<string, string> = {
  completed: 'bg-green-100 text-green-800',
  generating: 'bg-yellow-100 text-yellow-800',
  failed: 'bg-red-100 text-red-800',
}

type BoardColumn = 'daily' | 'weekly' | 'monthly'

const BOARD_COLUMNS: { id: BoardColumn; label: string; hint: string }[] = [
  { id: 'daily', label: 'Daily', hint: '일별 요약 리포트' },
  { id: 'weekly', label: 'Weekly', hint: '주간 요약 리포트' },
  { id: 'monthly', label: 'Monthly', hint: '월간 종합 리포트' },
]

export function ReportsPage() {
  const navigate = useNavigate()
  const [page, setPage] = useState(1)
  const [category, setCategory] = useState<ReportCategory>('all')
  const [dateInputs, setDateInputs] = useState<Record<string, string>>({})
  const queryClient = useQueryClient()

  const { data: reportsData, isLoading } = useQuery<ApiResponse<Report[]>>({
    queryKey: ['reports', category, page],
    queryFn: () => {
      const params = new URLSearchParams({ page: String(page), size: '100' })
      if (category !== 'all') params.set('report_type', category)
      return api.get(`/reports?${params.toString()}`).then((res) => res.data)
    },
  })

  const generateMutation = useMutation({
    mutationFn: ({ reportType, date }: { reportType: string; date?: string }) => {
      const payload: Record<string, string> = { report_type: reportType }
      if (date) payload.date = date
      return api.post('/reports', payload)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reports'] })
    },
  })

  const reports = reportsData?.data ?? []
  const meta = reportsData?.meta
  const totalPages = meta ? Math.ceil(meta.total / 100) : 1

  const selectCategory = (key: ReportCategory) => {
    setCategory(key)
    setPage(1)
  }

  const columnReports = (id: BoardColumn): Report[] =>
    reports
      .filter((r) => r.report_type === id)
      .sort((a, b) => (a.period_start < b.period_start ? 1 : -1))

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Reports</h1>
        <div className="flex flex-wrap gap-4">
          {(['daily', 'weekly', 'monthly'] as const).map((key) => (
            <div key={key} className="flex items-center gap-2">
              <input
                type="date"
                aria-label={`${key} report date`}
                value={dateInputs[key] ?? ''}
                onChange={(e) =>
                  setDateInputs((prev) => ({ ...prev, [key]: e.target.value }))
                }
                className="h-9 px-2 rounded-md border border-border bg-background text-sm"
              />
              <Button
                variant={key === 'monthly' ? 'default' : 'outline'}
                disabled={generateMutation.isPending}
                onClick={() =>
                  generateMutation.mutate({ reportType: key, date: dateInputs[key] || undefined })
                }
              >
                Generate {key[0].toUpperCase() + key.slice(1)}
              </Button>
            </div>
          ))}
        </div>
      </div>

      <div className="flex gap-2">
        {CATEGORY_ORDER.map((key) => (
          <Button
            key={key}
            variant={category === key ? 'default' : 'outline'}
            size="sm"
            onClick={() => selectCategory(key)}
          >
            {CATEGORY_LABELS[key]}
          </Button>
        ))}
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center h-64">Loading...</div>
      ) : reports.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">
            No reports yet. Click a generate button to create one.
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 items-start">
          {BOARD_COLUMNS.map((col) => {
            const items = columnReports(col.id)
            return (
              <div
                key={col.id}
                className="rounded-xl border border-border bg-muted/40 p-3 min-h-[240px] flex flex-col"
              >
                <div className="flex items-center justify-between px-1 pb-3">
                  <div>
                    <div className="text-sm font-semibold">
                      {col.label}
                      <span className="ml-2 text-[10px] font-normal text-muted-foreground">
                        {col.hint}
                      </span>
                    </div>
                  </div>
                  <span className="px-2 py-0.5 rounded-full bg-background border border-border text-[10px] text-muted-foreground">
                    {items.length}
                  </span>
                </div>
                <div className="space-y-2 flex-1">
                  {items.length === 0 ? (
                    <div className="rounded-lg border border-dashed border-border p-4 text-center text-[11px] text-muted-foreground">
                      No {col.label.toLowerCase()} reports
                    </div>
                  ) : (
                    items.map((report) => (
                      <Card
                        key={report.id}
                        className="cursor-pointer hover:shadow-md transition-shadow"
                        onClick={() => navigate(`/reports/${report.id}`)}
                      >
                        <CardHeader className="px-3 py-2.5 pb-1">
                          <CardTitle className="text-sm leading-snug">
                            {report.title}
                          </CardTitle>
                        </CardHeader>
                        <CardContent className="px-3 pb-2.5 space-y-1.5">
                          <div className="flex items-center justify-between">
                            <span
                              className={`px-2 py-0.5 rounded-full text-[10px] ${
                                STATUS_STYLES[report.status] ?? 'bg-slate-100 text-slate-700'
                              }`}
                            >
                              {report.status}
                            </span>
                            <span className="text-[10px] text-muted-foreground">
                              {new Date(report.generated_at).toLocaleDateString()}
                            </span>
                          </div>
                          <div className="text-[11px] text-muted-foreground">
                            {new Date(report.period_start).toLocaleDateString()} ~{' '}
                            {new Date(report.period_end).toLocaleDateString()}
                          </div>
                          {report.summary && (
                            <div className="text-[11px] text-muted-foreground line-clamp-2">
                              {report.summary}
                            </div>
                          )}
                        </CardContent>
                      </Card>
                    ))
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {meta && meta.total > 100 && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            Showing {reports.length} of {meta.total} reports
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={page <= 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <span className="px-3 py-1 text-xs">
              {page} / {totalPages}
            </span>
            <Button
              variant="outline"
              size="sm"
              disabled={page >= totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
