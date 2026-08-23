import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Report, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function ReportsPage() {
  const [page, setPage] = useState(1)
  const queryClient = useQueryClient()

  const { data: reportsData, isLoading } = useQuery<ApiResponse<Report[]>>({
    queryKey: ['reports', page],
    queryFn: () => api.get(`/reports?page=${page}&size=20`).then((res) => res.data),
  })

  const generateMutation = useMutation({
    mutationFn: (reportType: string) => api.post('/reports', { report_type: reportType }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reports'] })
    },
  })

  const reports = reportsData?.data ?? []
  const meta = reportsData?.meta
  const totalPages = meta ? Math.ceil(meta.total / 20) : 1

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Reports</h1>
        <div className="flex gap-2">
          <Button
            variant="outline"
            onClick={() => generateMutation.mutate('daily')}
            disabled={generateMutation.isPending}
          >
            Generate Daily
          </Button>
          <Button
            variant="outline"
            onClick={() => generateMutation.mutate('weekly')}
            disabled={generateMutation.isPending}
          >
            Generate Weekly
          </Button>
          <Button
            onClick={() => generateMutation.mutate('monthly')}
            disabled={generateMutation.isPending}
          >
            Generate Monthly
          </Button>
        </div>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center h-64">Loading...</div>
      ) : (
        <div className="grid gap-4">
          {reports.length === 0 ? (
            <Card>
              <CardContent className="py-8 text-center text-muted-foreground">
                No reports generated yet. Click a generate button to create your first report.
              </CardContent>
            </Card>
          ) : (
            reports.map((report) => (
              <Card key={report.id}>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-lg">{report.title}</CardTitle>
                  <span className={`px-2 py-1 text-xs rounded-full ${
                    report.status === 'completed' ? 'bg-green-100 text-green-800' :
                    report.status === 'generating' ? 'bg-yellow-100 text-yellow-800' :
                    'bg-red-100 text-red-800'
                  }`}>
                    {report.status}
                  </span>
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
