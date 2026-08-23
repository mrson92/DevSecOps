import { useQuery } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { DashboardStats, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { TimelineChart } from '../components/TimelineChart'
import { SeverityChart } from '../components/SeverityChart'
import { TopRulesChart } from '../components/TopRulesChart'
import { TopIpsChart } from '../components/TopIpsChart'

export function DashboardPage() {
  const { data: stats, isLoading: statsLoading } = useQuery<ApiResponse<DashboardStats>>({
    queryKey: ['dashboard-stats'],
    queryFn: () => api.get('/dashboard/stats').then((res) => res.data),
  })

  const { data: timeline } = useQuery<ApiResponse<Array<{ time: string; count: number }>>>({
    queryKey: ['dashboard-timeline'],
    queryFn: () => api.get('/dashboard/timeline').then((res) => res.data),
    refetchInterval: 60000,
  })

  const { data: topRules } = useQuery<ApiResponse<Array<{ id: string; name: string; severity: string; count: number }>>>({
    queryKey: ['dashboard-top-rules'],
    queryFn: () => api.get('/dashboard/top-rules').then((res) => res.data),
  })

  const { data: topIps } = useQuery<ApiResponse<Array<{ ip: string; count: number }>>>({
    queryKey: ['dashboard-top-ips'],
    queryFn: () => api.get('/dashboard/top-ips').then((res) => res.data),
  })

  if (statsLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  const data = stats?.data

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Dashboard</h1>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Detections</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{data?.total_detections ?? 0}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Open Detections</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-orange-500">{data?.open_detections ?? 0}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Rules</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-500">{data?.active_rules ?? 0}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Critical Issues</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-500">{data?.critical_count ?? 0}</div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>24h Timeline</CardTitle>
          </CardHeader>
          <CardContent>
            <TimelineChart data={timeline?.data ?? []} />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Severity Distribution</CardTitle>
          </CardHeader>
          <CardContent>
            <SeverityChart
              critical={data?.critical_count ?? 0}
              high={data?.high_count ?? 0}
              medium={data?.medium_count ?? 0}
              low={data?.low_count ?? 0}
            />
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Top Rules (7 days)</CardTitle>
          </CardHeader>
          <CardContent>
            <TopRulesChart data={topRules?.data ?? []} />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Top IPs (7 days)</CardTitle>
          </CardHeader>
          <CardContent>
            <TopIpsChart data={topIps?.data ?? []} />
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
