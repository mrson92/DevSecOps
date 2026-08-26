import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { AiAgent, AiAgentRun, Persona, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { AgentFormDialog } from '../components/AgentFormDialog'
import { PersonaManager } from '../components/PersonaManager'

export function AgentsPage() {
  const [dialogOpen, setDialogOpen] = useState(false)
  const [personaManagerOpen, setPersonaManagerOpen] = useState(false)
  const [selectedAgentRuns, setSelectedAgentRuns] = useState<string | null>(null)
  const queryClient = useQueryClient()

  const { data: agentsData, isLoading: agentsLoading } = useQuery<ApiResponse<AiAgent[]>>({
    queryKey: ['agents'],
    queryFn: () => api.get('/agents').then((res) => res.data),
  })

  const { data: personasData } = useQuery<ApiResponse<Persona[]>>({
    queryKey: ['personas'],
    queryFn: () => api.get('/personas').then((res) => res.data),
  })

  const { data: runsData, isLoading: runsLoading } = useQuery<ApiResponse<AiAgentRun[]>>({
    queryKey: ['agentRuns', selectedAgentRuns],
    queryFn: () => api.get(`/agents/${selectedAgentRuns}/runs`).then((res) => res.data),
    enabled: !!selectedAgentRuns,
  })

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.delete(`/agents/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] })
    },
  })

  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      api.put(`/agents/${id}`, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] })
    },
  })

  const runMutation = useMutation({
    mutationFn: (id: string) => api.post(`/agents/${id}/run`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agentRuns'] })
    },
  })

  if (agentsLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  const agents = agentsData?.data ?? []
  const personas = personasData?.data ?? []
  const runs = runsData?.data ?? []

  const getPersonaName = (personaId: string) => {
    const persona = personas.find((p) => p.id === personaId)
    return persona?.name ?? 'Unknown'
  }

  const agentTypeColors: Record<string, string> = {
    analyzer: 'bg-blue-100 text-blue-800',
    responder: 'bg-red-100 text-red-800',
    investigator: 'bg-purple-100 text-purple-800',
    reporter: 'bg-green-100 text-green-800',
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">AI Agents</h1>
          <p className="text-muted-foreground mt-1">
            AI 기반 보안 분석 에이전트를 관리합니다
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => setPersonaManagerOpen(true)}>
            Manage Personas
          </Button>
          <Button onClick={() => setDialogOpen(true)}>Create Agent</Button>
        </div>
      </div>

      <div className="grid gap-4">
        {agents.length === 0 ? (
          <Card>
            <CardContent className="py-8 text-center text-muted-foreground">
              No AI agents configured yet. Create your first agent to start automated security analysis.
            </CardContent>
          </Card>
        ) : (
          agents.map((agent) => (
            <Card key={agent.id} className="hover:bg-muted/50 transition-colors">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <div className="flex items-center gap-3">
                  <CardTitle className="text-lg">{agent.name}</CardTitle>
                  <span
                    className={`px-2 py-1 text-xs rounded-full ${
                      agentTypeColors[agent.agent_type] ?? 'bg-gray-100 text-gray-800'
                    }`}
                  >
                    {agent.agent_type}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => runMutation.mutate(agent.id)}
                    disabled={runMutation.isPending}
                  >
                     Run Now
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      toggleMutation.mutate({
                        id: agent.id,
                        enabled: !agent.enabled,
                      })
                    }
                  >
                    {agent.enabled ? ' Disable' : ' Enable'}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="text-red-600 hover:text-red-700"
                    onClick={() => {
                      if (confirm('Are you sure you want to delete this agent?')) {
                        deleteMutation.mutate(agent.id)
                      }
                    }}
                  >
                    Delete
                  </Button>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">{agent.description}</p>
                <div className="mt-3 flex flex-wrap gap-4 text-xs text-muted-foreground">
                  <span>Persona: {getPersonaName(agent.persona_id)}</span>
                  <span>Status: {agent.enabled ? ' Active' : ' Inactive'}</span>
                  {agent.schedule && <span>Schedule: {agent.schedule}</span>}
                  <span>Created: {new Date(agent.created_at).toLocaleDateString()}</span>
                </div>
                <div className="mt-3">
                  <Button
                    variant="link"
                    size="sm"
                    className="p-0 h-auto"
                    onClick={() => setSelectedAgentRuns(selectedAgentRuns === agent.id ? null : agent.id)}
                  >
                    {selectedAgentRuns === agent.id ? 'Hide Runs' : 'View Run History'}
                  </Button>
                </div>
                {selectedAgentRuns === agent.id && (
                  <div className="mt-4 border-t pt-4">
                    {runsLoading ? (
                      <div className="text-sm text-muted-foreground">Loading runs...</div>
                    ) : runs.length === 0 ? (
                      <div className="text-sm text-muted-foreground">No runs yet</div>
                    ) : (
                      <div className="space-y-2">
                        {runs.slice(0, 5).map((run) => (
                          <div key={run.id} className="flex items-center justify-between text-sm">
                            <div className="flex items-center gap-2">
                              <span
                                className={`w-2 h-2 rounded-full ${
                                  run.status === 'completed' ? 'bg-green-500' : 'bg-red-500'
                                }`}
                              />
                              <span className="text-muted-foreground">
                                {new Date(run.started_at).toLocaleString()}
                              </span>
                            </div>
                            <div className="flex items-center gap-4">
                              {run.token_usage && <span>{run.token_usage} tokens</span>}
                              {run.error_message && (
                                <span className="text-red-600 truncate max-w-[200px]">
                                  {run.error_message}
                                </span>
                              )}
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </CardContent>
            </Card>
          ))
        )}
      </div>

      <AgentFormDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        personas={personas}
      />

      <PersonaManager
        open={personaManagerOpen}
        onOpenChange={setPersonaManagerOpen}
      />
    </div>
  )
}
