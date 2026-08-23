import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import api from '@/shared/lib/api'
import type { Rule, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { RuleFormDialog } from '../components/RuleFormDialog'

export function RulesPage() {
  const [page, setPage] = useState(1)
  const [dialogOpen, setDialogOpen] = useState(false)
  const size = 20

  const { data: rulesData, isLoading } = useQuery<ApiResponse<Rule[]>>({
    queryKey: ['rules', page],
    queryFn: () => api.get(`/rules?page=${page}&size=${size}`).then((res) => res.data),
  })

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  const rules = rulesData?.data ?? []
  const meta = rulesData?.meta
  const totalPages = meta ? Math.ceil(meta.total / size) : 1

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Rules</h1>
        <Button onClick={() => setDialogOpen(true)}>Create Rule</Button>
      </div>

      <div className="grid gap-4">
        {rules.length === 0 ? (
          <Card>
            <CardContent className="py-8 text-center text-muted-foreground">
              No rules configured yet. Create your first rule to start detecting anomalies.
            </CardContent>
          </Card>
        ) : (
          rules.map((rule) => (
            <Link key={rule.id} to={`/rules/${rule.id}`}>
              <Card className="hover:bg-muted/50 transition-colors cursor-pointer">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-lg">{rule.name}</CardTitle>
                  <span
                    className={`px-2 py-1 text-xs rounded-full ${
                      rule.severity === 'critical'
                        ? 'bg-red-100 text-red-800'
                        : rule.severity === 'high'
                        ? 'bg-orange-100 text-orange-800'
                        : rule.severity === 'medium'
                        ? 'bg-yellow-100 text-yellow-800'
                        : 'bg-green-100 text-green-800'
                    }`}
                  >
                    {rule.severity}
                  </span>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">{rule.description}</p>
                  <div className="mt-2 flex gap-2 text-xs text-muted-foreground">
                    <span>Type: {rule.rule_type}</span>
                    <span>•</span>
                    <span>Window: {rule.window_sec}s</span>
                    <span>•</span>
                    <span>Version: {rule.version}</span>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))
        )}
      </div>

      {meta && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            Showing {rules.length} of {meta.total} rules
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

      <RuleFormDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </div>
  )
}
