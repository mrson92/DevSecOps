import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Report, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

type ReportCategory = 'all' | 'daily' | 'weekly' | 'monthly'

const CATEGORY_LABELS: Record<ReportCategory, string> = {
  all: 'All',
  daily: 'Daily',
  weekly: 'Weekly',
  monthly: 'Monthly',
}

const CATEGORY_ORDER: ReportCategory[] = ['all', 'daily', 'weekly', 'monthly']

export function ReportsPage() {
  const [page, setPage] = useState(1)
  const [category, setCategory] = useState<ReportCategory>('all')
  const [dateInputs, setDateInputs] = useState<Record<string, string>>({})
  const queryClient = useQueryClient()

  const { data: reportsData, isLoading } = useQuery<ApiResponse<Report[]>>({
    queryKey: ['reports', category, page],
    queryFn: () => {
      const params = new URLSearchParams({ page: String(page), size: '20' })
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

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.delete(`/reports/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reports'] })
    },
  })

  const reports = reportsData?.data ?? []
  const meta = reportsData?.meta
  const totalPages = meta ? Math.ceil(meta.total / 20) : 1

  const selectCategory = (key: ReportCategory) => {
    setCategory(key)
    setPage(1)
  }

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
      ) : (
        <div className="grid gap-4">
          {reports.length === 0 ? (
            <Card>
              <CardContent className="py-8 text-center text-muted-foreground">
                No reports in this category yet. Click a generate button to create one.
              </CardContent>
            </Card>
          ) : (
            reports.map((report) => (
              <Card key={report.id}>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-lg">{report.title}</CardTitle>
                  <div className="flex items-center gap-2">
                    <span className={`px-2 py-1 text-xs rounded-full ${
                      report.status === 'completed' ? 'bg-green-100 text-green-800' :
                      report.status === 'generating' ? 'bg-yellow-100 text-yellow-800' :
                      'bg-red-100 text-red-800'
                    }`}>
                      {report.status}
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs text-red-600 hover:text-red-700 hover:bg-red-50"
                      disabled={deleteMutation.isPending}
                      onClick={() => deleteMutation.mutate(report.id)}
                    >
                      Delete
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="flex gap-4 text-sm text-muted-foreground">
                    <span>Type: {report.report_type}</span>
                    <span>Period: {new Date(report.period_start).toLocaleDateString()} ~ {new Date(report.period_end).toLocaleDateString()}</span>
                    <span>Generated: {new Date(report.generated_at).toLocaleString()}</span>
                  </div>
                  {report.summary && (
                    <pre className="mt-3 p-3 bg-muted rounded-md text-xs overflow-x-auto">
                      {report.summary}
                    </pre>
                  )}
                </CardContent>
              </Card>
            ))
          )}
        </div>
      )}

      {meta && meta.total > 20 && (
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
