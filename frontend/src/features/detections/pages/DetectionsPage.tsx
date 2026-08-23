import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import api from '@/shared/lib/api'
import type { Detection, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

export function DetectionsPage() {
  const { data: detectionsData, isLoading } = useQuery<ApiResponse<Detection[]>>({
    queryKey: ['detections'],
    queryFn: () => api.get('/detections').then((res) => res.data),
  })

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  const detections = detectionsData?.data ?? []

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Detections</h1>

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
    </div>
  )
}
