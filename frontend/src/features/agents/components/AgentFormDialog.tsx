import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Persona, CreateAgentRequest } from '@/shared/types'

interface AgentFormDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  personas: Persona[]
}

export function AgentFormDialog({ open, onOpenChange, personas }: AgentFormDialogProps) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [personaId, setPersonaId] = useState('')
  const [agentType, setAgentType] = useState<string>('analyzer')
  const [schedule, setSchedule] = useState('')
  const queryClient = useQueryClient()

  const createMutation = useMutation({
    mutationFn: (data: CreateAgentRequest) => api.post('/agents', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] })
      resetForm()
      onOpenChange(false)
    },
  })

  const resetForm = () => {
    setName('')
    setDescription('')
    setPersonaId('')
    setAgentType('analyzer')
    setSchedule('')
  }

  const handleSubmit = () => {
    if (!name.trim() || !personaId) return

    createMutation.mutate({
      name: name.trim(),
      description: description.trim() || undefined,
      persona_id: personaId,
      agent_type: agentType,
      schedule: schedule.trim() || undefined,
    })
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="fixed inset-0 bg-black/50" onClick={() => onOpenChange(false)} />
      <div className="relative bg-card rounded-lg shadow-lg w-full max-w-[500px] mx-4 p-6 space-y-4">
        <h2 className="text-lg font-semibold">Create AI Agent</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">Agent Name *</label>
          <input
            type="text"
            value={name}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
            placeholder="e.g., Security Analyzer"
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Description</label>
          <input
            type="text"
            value={description}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDescription(e.target.value)}
            placeholder="What does this agent do?"
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Persona *</label>
          <select
            value={personaId}
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setPersonaId(e.target.value)}
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
          >
            <option value="">Select a persona</option>
            {personas.map((persona) => (
              <option key={persona.id} value={persona.id}>
                {persona.name} ({persona.model})
              </option>
            ))}
          </select>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Agent Type</label>
          <select
            value={agentType}
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setAgentType(e.target.value)}
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
          >
            <option value="analyzer">Analyzer - Log analysis</option>
            <option value="responder">Responder - Incident response</option>
            <option value="investigator">Investigator - Threat hunting</option>
            <option value="reporter">Reporter - Report generation</option>
          </select>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Schedule (optional)</label>
          <input
            type="text"
            value={schedule}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSchedule(e.target.value)}
            placeholder="e.g., */30 * * * * (every 30 min)"
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
          />
        </div>

        <div className="flex justify-end gap-2 pt-2">
          <button
            onClick={() => onOpenChange(false)}
            className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            disabled={!name.trim() || !personaId}
            className="px-4 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            Create Agent
          </button>
        </div>
      </div>
    </div>
  )
}
