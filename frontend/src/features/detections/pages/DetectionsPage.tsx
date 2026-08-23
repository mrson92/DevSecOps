import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import api from '@/shared/lib/api'
import type { Detection, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

interface DetectionFilters {
  severity?: string
  status?: string
  page: number
  size: number
}

export function DetectionsPage() {
  const [filters, setFilters] = useState<DetectionFilters>({ page: 1, size: 20 })
  const [showFilters, setShowFilters] = useState(false)

  const { data: detectionsData, isLoading } = useQuery<ApiResponse<Detection[]>>({
    queryKey: ['detections', filters],
    queryFn: () => {
      const params = new URLSearchParams()
      params.set('page', String(filters.page))
      params.set('size', String(filters.size))
      if (filters.severity) params.set('severity', filters.severity)
      if (filters.status) params.set('status', filters.status)
      return api.get(`/detections?${params.toString()}`).then((res) => res.data)
    },
  })

  const detections = detectionsData?.data ?? []
  const meta = detectionsData?.meta
  const totalPages = meta ? Math.ceil(meta.total / filters.size) : 1

  const updateFilter = (key: keyof DetectionFilters, value: string | number | undefined) => {
    setFilters((prev) => ({ ...prev, [key]: value, page: 1 }))
  }

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
              <div className="flex items-end">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setFilters({ page: 1, size: 20 })}
                >
                  Reset Filters
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {isLoading ? (
        <div className="flex items-center justify-center h-64">Loading...</div>
      ) : (
        <div className="grid gap-4">
          {detections.length === 0 ? (
            <Card>
              <CardContent className="py-8 text-center text-muted-foreground">
                No detections found. All clear!
              </CardContent>
            </Card>
          ) : (
            detections.map((detection) => (
              <Link key={detection.id} to={`/detections/${detection.id}`}>
                <Card className="hover:bg-muted/50 transition-colors cursor-pointer">
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-lg">Detection #{detection.id.slice(0, 8)}</CardTitle>
                    <span className={`px-2 py-1 text-xs rounded-full ${
                      detection.status === 'open' ? 'bg-red-100 text-red-800' :
                      detection.status === 'acknowledged' ? 'bg-yellow-100 text-yellow-800' :
                      detection.status === 'investigating' ? 'bg-blue-100 text-blue-800' :
                      'bg-green-100 text-green-800'
                    }`}>
                      {detection.status}
                    </span>
                  </CardHeader>
                  <CardContent>
                    <div className="flex gap-4 text-sm text-muted-foreground">
                      <span>Matched: {detection.matched_count}</span>
                      <span>Rule: {detection.rule_id.slice(0, 8)}</span>
                      <span>Detected: {new Date(detection.detected_at).toLocaleString()}</span>
                    </div>
                  </CardContent>
                </Card>
              </Link>
            ))
          )}
        </div>
      )}

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
