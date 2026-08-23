import { useParams, useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Detection, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function DetectionDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

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
          <p className="text-muted-foreground">Detected at {new Date(detection.detected_at).toLocaleString()}</p>
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
            <CardTitle className="text-sm font-medium">Rule Version</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">v{detection.rule_version}</div>
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
          <CardTitle>Context</CardTitle>
        </CardHeader>
        <CardContent>
          <pre className="p-4 bg-muted rounded-md overflow-x-auto text-sm">
            {JSON.stringify(detection.context, null, 2) ?? 'No context available'}
          </pre>
        </CardContent>
      </Card>
    </div>
  )
}
