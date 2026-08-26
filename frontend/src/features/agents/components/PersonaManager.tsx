import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Persona, ApiResponse, CreatePersonaRequest, UpdatePersonaRequest } from '@/shared/types'

interface PersonaManagerProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function PersonaManager({ open, onOpenChange }: PersonaManagerProps) {
  const [editingPersona, setEditingPersona] = useState<Persona | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [systemPrompt, setSystemPrompt] = useState('')
  const [model, setModel] = useState('gpt-4')
  const [temperature, setTemperature] = useState('0.7')
  const [maxTokens, setMaxTokens] = useState('4096')
  const queryClient = useQueryClient()

  const { data: personasData, isLoading } = useQuery<ApiResponse<Persona[]>>({
    queryKey: ['personas'],
    queryFn: () => api.get('/personas').then((res) => res.data),
    enabled: open,
  })

  const createMutation = useMutation({
    mutationFn: (data: CreatePersonaRequest) => api.post('/personas', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['personas'] })
      resetForm()
      setShowForm(false)
    },
  })

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdatePersonaRequest }) =>
      api.put(`/personas/${id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['personas'] })
      resetForm()
      setEditingPersona(null)
      setShowForm(false)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.delete(`/personas/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['personas'] })
    },
  })

  const resetForm = () => {
    setName('')
    setDescription('')
    setSystemPrompt('')
    setModel('gpt-4')
    setTemperature('0.7')
    setMaxTokens('4096')
  }

  const handleEdit = (persona: Persona) => {
    setEditingPersona(persona)
    setName(persona.name)
    setDescription(persona.description ?? '')
    setSystemPrompt(persona.system_prompt)
    setModel(persona.model)
    setTemperature(String(persona.temperature))
    setMaxTokens(String(persona.max_tokens))
    setShowForm(true)
  }

  const handleSubmit = () => {
    if (!name.trim() || !systemPrompt.trim()) return

    const data: CreatePersonaRequest = {
      name: name.trim(),
      description: description.trim() || undefined,
      system_prompt: systemPrompt.trim(),
      model,
      temperature: parseFloat(temperature),
      max_tokens: parseInt(maxTokens, 10),
    }

    if (editingPersona) {
      updateMutation.mutate({ id: editingPersona.id, data })
    } else {
      createMutation.mutate(data)
    }
  }

  const personas = personasData?.data ?? []

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="fixed inset-0 bg-black/50" onClick={() => onOpenChange(false)} />
      <div className="relative bg-card rounded-lg shadow-lg w-full max-w-[700px] mx-4 p-6 max-h-[80vh] overflow-y-auto">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">Persona Management</h2>
          {!showForm && (
            <button
              onClick={() => {
                resetForm()
                setEditingPersona(null)
                setShowForm(true)
              }}
              className="px-4 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90"
            >
              New Persona
            </button>
          )}
        </div>

        {showForm ? (
          <div className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Name *</label>
              <input
                type="text"
                value={name}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
                placeholder="e.g., Security Analyst"
                className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Description</label>
              <input
                type="text"
                value={description}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDescription(e.target.value)}
                placeholder="What does this persona do?"
                className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">System Prompt *</label>
              <textarea
                value={systemPrompt}
                onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setSystemPrompt(e.target.value)}
                placeholder="You are a security analyst..."
                rows={6}
                className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              />
            </div>
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Model</label>
                <select
                  value={model}
                  onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setModel(e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                >
                  <option value="gpt-4">GPT-4</option>
                  <option value="gpt-4-turbo">GPT-4 Turbo</option>
                  <option value="gpt-3.5-turbo">GPT-3.5 Turbo</option>
                  <option value="claude-3-opus">Claude 3 Opus</option>
                  <option value="claude-3-sonnet">Claude 3 Sonnet</option>
                </select>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Temperature</label>
                <input
                  type="number"
                  min="0"
                  max="2"
                  step="0.1"
                  value={temperature}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => setTemperature(e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Max Tokens</label>
                <input
                  type="number"
                  min="256"
                  max="128000"
                  step="256"
                  value={maxTokens}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => setMaxTokens(e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                />
              </div>
            </div>
            <div className="flex justify-end gap-2 pt-2">
              <button
                onClick={() => setShowForm(false)}
                className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted"
              >
                Cancel
              </button>
              <button
                onClick={handleSubmit}
                disabled={!name.trim() || !systemPrompt.trim()}
                className="px-4 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                {editingPersona ? 'Update Persona' : 'Create Persona'}
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            {isLoading ? (
              <div className="text-center py-4 text-muted-foreground">Loading...</div>
            ) : personas.length === 0 ? (
              <div className="text-center py-4 text-muted-foreground">
                No personas created yet.
              </div>
            ) : (
              personas.map((persona) => (
                <div
                  key={persona.id}
                  className="border border-border rounded-lg p-4 space-y-2"
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="font-medium">{persona.name}</h3>
                      <p className="text-sm text-muted-foreground">
                        {persona.model} | Temp: {persona.temperature} | Max: {persona.max_tokens}
                      </p>
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={() => handleEdit(persona)}
                        className="px-3 py-1 text-sm rounded-md border border-border hover:bg-muted"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => {
                          if (confirm(`Delete persona "${persona.name}"?`)) {
                            deleteMutation.mutate(persona.id)
                          }
                        }}
                        className="px-3 py-1 text-sm rounded-md border border-red-200 text-red-600 hover:bg-red-50"
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                  <p className="text-sm text-muted-foreground line-clamp-2">
                    {persona.description || persona.system_prompt}
                  </p>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  )
}
