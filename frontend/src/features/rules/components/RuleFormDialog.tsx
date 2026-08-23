import { useState, useEffect } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { Rule } from '@/shared/types'
import { Button } from '@/components/ui/button'

interface RuleFormDialogProps {
  open: boolean
  onClose: () => void
  rule?: Rule | null
}

interface RuleFormData {
  name: string
  description: string
  severity: string
  rule_type: string
  condition: string
  window_sec: number
  slide_sec: number
}

const defaultForm: RuleFormData = {
  name: '',
  description: '',
  severity: 'medium',
  rule_type: 'threshold',
  condition: '',
  window_sec: 300,
  slide_sec: 60,
}

export function RuleFormDialog({ open, onClose, rule }: RuleFormDialogProps) {
  const [form, setForm] = useState<RuleFormData>(defaultForm)
  const queryClient = useQueryClient()

  useEffect(() => {
    if (rule) {
      setForm({
        name: rule.name,
        description: rule.description ?? '',
        severity: rule.severity,
        rule_type: rule.rule_type,
        condition: rule.condition,
        window_sec: rule.window_sec,
        slide_sec: rule.slide_sec,
      })
    } else {
      setForm(defaultForm)
    }
  }, [rule, open])

  const createMutation = useMutation({
    mutationFn: (data: RuleFormData) => api.post('/rules', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rules'] })
      onClose()
    },
  })

  const updateMutation = useMutation({
    mutationFn: (data: RuleFormData) => api.put(`/rules/${rule?.id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rules'] })
      queryClient.invalidateQueries({ queryKey: ['rule', rule?.id] })
      onClose()
    },
  })

  const handleSubmit = () => {
    if (!form.name.trim() || !form.condition.trim()) return
    if (rule) {
      updateMutation.mutate(form)
    } else {
      createMutation.mutate(form)
    }
  }

  const isPending = createMutation.isPending || updateMutation.isPending

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="fixed inset-0 bg-black/50" onClick={onClose} />
      <div className="relative bg-card rounded-xl shadow-xl w-full max-w-2xl max-h-[90vh] overflow-y-auto p-6 space-y-4">
        <h2 className="text-xl font-bold">{rule ? 'Edit Rule' : 'Create Rule'}</h2>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <label className="text-sm font-medium">Name *</label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              placeholder="Rule name"
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Severity</label>
            <select
              value={form.severity}
              onChange={(e) => setForm({ ...form, severity: e.target.value })}
              className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
            >
              <option value="critical">Critical</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
              <option value="info">Info</option>
            </select>
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Rule Type</label>
            <select
              value={form.rule_type}
              onChange={(e) => setForm({ ...form, rule_type: e.target.value })}
              className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
            >
              <option value="threshold">Threshold</option>
              <option value="pattern">Pattern</option>
              <option value="sequence">Sequence</option>
              <option value="composite">Composite</option>
            </select>
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Description</label>
            <input
              type="text"
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
              className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              placeholder="Optional description"
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Window (seconds)</label>
            <input
              type="number"
              value={form.window_sec}
              onChange={(e) => setForm({ ...form, window_sec: parseInt(e.target.value) || 300 })}
              className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              min={1}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Slide (seconds)</label>
            <input
              type="number"
              value={form.slide_sec}
              onChange={(e) => setForm({ ...form, slide_sec: parseInt(e.target.value) || 60 })}
              className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
              min={1}
            />
          </div>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Condition (CEL Expression) *</label>
          <textarea
            value={form.condition}
            onChange={(e) => setForm({ ...form, condition: e.target.value })}
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm font-mono h-32 resize-none"
            placeholder='e.g. size(filter(logs, log -> log.http.response.status_code >= 400)) > 50'
          />
        </div>

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="outline" onClick={onClose} disabled={isPending}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={isPending || !form.name.trim() || !form.condition.trim()}>
            {isPending ? 'Saving...' : rule ? 'Update Rule' : 'Create Rule'}
          </Button>
        </div>
      </div>
    </div>
  )
}
