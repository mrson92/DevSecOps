import { useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Rule, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function RuleDetailPage() {
  const { id } = useParams<{ id: string }>()

  const { data: ruleData, isLoading } = useQuery<ApiResponse<Rule>>({
    queryKey: ['rule', id],
    queryFn: () => api.get(`/rules/${id}`).then((res) => res.data),
    enabled: !!id,
  })

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  const rule = ruleData?.data

  if (!rule) {
    return <div className="text-center py-8">Rule not found</div>
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{rule.name}</h1>
          <p className="text-muted-foreground">{rule.description}</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline">Test Rule</Button>
          <Button>Edit Rule</Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Severity</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`px-2 py-1 text-xs rounded-full ${
              rule.severity === 'critical' ? 'bg-red-100 text-red-800' :
              rule.severity === 'high' ? 'bg-orange-100 text-orange-800' :
              rule.severity === 'medium' ? 'bg-yellow-100 text-yellow-800' :
              'bg-green-100 text-green-800'
            }`}>
              {rule.severity}
            </span>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Type</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{rule.rule_type}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Window</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{rule.window_sec}s</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Version</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">v{rule.version}</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Condition</CardTitle>
        </CardHeader>
        <CardContent>
          <pre className="p-4 bg-muted rounded-md overflow-x-auto text-sm">
            {rule.condition}
          </pre>
        </CardContent>
      </Card>
    </div>
  )
}
