import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Rule, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { RuleFormDialog } from '../components/RuleFormDialog'
import { RuleTestPanel } from '../components/RuleTestPanel'

function parseStringArray(value: string[] | string | null | undefined): string[] {
  if (!value) return []
  if (Array.isArray(value)) return value
  try {
    const parsed = JSON.parse(value)
    return Array.isArray(parsed) ? parsed.map(String) : []
  } catch {
    return []
  }
}

export function RuleDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [editOpen, setEditOpen] = useState(false)

  const { data: ruleData, isLoading, isError, error } = useQuery<ApiResponse<Rule>>({
    queryKey: ['rule', id],
    queryFn: () => api.get(`/rules/${id}`).then((res) => res.data),
    enabled: !!id,
  })

  const deleteMutation = useMutation({
    mutationFn: () => api.delete(`/rules/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rules'] })
      navigate('/rules')
    },
  })

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  if (isError) {
    return (
      <div className="text-center py-8 space-y-3">
        <p className="text-red-600 font-medium">Failed to load rule</p>
        <p className="text-sm text-muted-foreground">{(error as Error).message}</p>
        <Button variant="outline" onClick={() => navigate('/rules')}>
          Back to Rules
        </Button>
      </div>
    )
  }

  const rule = ruleData?.data

  if (!rule) {
    return <div className="text-center py-8">Rule not found</div>
  }

  const tactics = parseStringArray(rule.mitre_tactics)
  const techniques = parseStringArray(rule.mitre_techniques)
  const tags = parseStringArray(rule.tags)

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{rule.name}</h1>
          <p className="text-muted-foreground">{rule.description}</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => setEditOpen(true)}>
            Edit Rule
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              if (confirm('Are you sure you want to delete this rule?')) {
                deleteMutation.mutate()
              }
            }}
            disabled={deleteMutation.isPending}
          >
            {deleteMutation.isPending ? 'Deleting...' : 'Delete'}
          </Button>
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

      {(tags.length || techniques.length || tactics.length) && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Tags & MITRE</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {tactics.length > 0 && (
              <div className="space-y-1">
                <span className="text-xs font-medium text-muted-foreground">Tactics</span>
                <div className="flex flex-wrap gap-2">
                  {tactics.map((t) => (
                    <span key={t} className="px-2 py-1 text-xs rounded-full bg-blue-50 text-blue-700 font-mono">{t}</span>
                  ))}
                </div>
              </div>
            )}
            {techniques.length > 0 && (
              <div className="space-y-1">
                <span className="text-xs font-medium text-muted-foreground">Techniques</span>
                <div className="flex flex-wrap gap-2">
                  {techniques.map((t) => (
                    <span key={t} className="px-2 py-1 text-xs rounded-full bg-indigo-50 text-indigo-700 font-mono">{t}</span>
                  ))}
                </div>
              </div>
            )}
            {tags.length > 0 && (
              <div className="space-y-1">
                <span className="text-xs font-medium text-muted-foreground">Tags</span>
                <div className="flex flex-wrap gap-2">
                  {tags.map((tag) => (
                    <span key={tag} className="px-2 py-1 text-xs rounded-full bg-slate-100 text-slate-700 font-mono">
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}

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

      <RuleTestPanel ruleId={rule.id} />

      <RuleFormDialog
        open={editOpen}
        onClose={() => setEditOpen(false)}
        rule={rule}
      />
    </div>
  )
}
