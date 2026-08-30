import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import type { Rule } from '@/shared/types'
import { Card, CardContent } from '@/components/ui/card'
import { parseStringArray, MITRE_TACTICS } from '@/shared/lib/mitre'

interface MitreGroupsViewProps {
  rules: Rule[]
  isLoading: boolean
}

interface TacticGroup {
  tactic: string
  name: string
  rules: Rule[]
}

function severityClass(severity: Rule['severity']): string {
  return severity === 'critical'
    ? 'bg-red-100 text-red-800'
    : severity === 'high'
    ? 'bg-orange-100 text-orange-800'
    : severity === 'medium'
    ? 'bg-yellow-100 text-yellow-800'
    : 'bg-green-100 text-green-800'
}

function RuleTable({ rules }: { rules: Rule[] }) {
  if (rules.length === 0) {
    return <p className="text-sm text-muted-foreground">No rules in this group.</p>
  }
  return (
    <div className="overflow-x-auto rounded-md border">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b bg-muted/50 text-left">
            <th className="px-3 py-2 font-medium">Rule</th>
            <th className="px-3 py-2 font-medium">Severity</th>
            <th className="px-3 py-2 font-medium">Type</th>
            <th className="px-3 py-2 font-medium">Techniques</th>
          </tr>
        </thead>
        <tbody>
          {rules.map((rule) => {
            const techniques = parseStringArray(rule.mitre_techniques)
            return (
              <tr key={rule.id} className="border-b last:border-0 hover:bg-muted/40">
                <td className="px-3 py-2">
                  <Link
                    to={`/rules/${rule.id}`}
                    className="font-medium text-primary underline underline-offset-4"
                  >
                    {rule.name}
                  </Link>
                </td>
                <td className="px-3 py-2">
                  <span className={`px-2 py-0.5 text-xs rounded-full ${severityClass(rule.severity)}`}>
                    {rule.severity}
                  </span>
                </td>
                <td className="px-3 py-2 text-muted-foreground">{rule.rule_type}</td>
                <td className="px-3 py-2">
                  <div className="flex flex-wrap gap-1">
                    {techniques.map((t) => (
                      <span
                        key={t}
                        className="px-1.5 py-0.5 text-xs rounded-full bg-indigo-50 text-indigo-700 font-mono"
                      >
                        {t}
                      </span>
                    ))}
                  </div>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

function TacticCard({ group }: { group: TacticGroup }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <Card>
      <CardContent className="p-4">
        <button
          className="w-full flex items-center justify-between gap-3 text-left"
          onClick={() => setExpanded((v) => !v)}
        >
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-mono text-xs text-blue-700">{group.tactic}</span>
              <span className="font-medium">{group.name}</span>
            </div>
            <div className="mt-1 text-sm text-muted-foreground">
              {group.rules.length} rule{group.rules.length > 1 ? 's' : ''}
            </div>
          </div>
          <span className="text-muted-foreground">{expanded ? '▾' : '▸'}</span>
        </button>
        {expanded && (
          <div className="mt-4">
            <RuleTable rules={group.rules} />
          </div>
        )}
      </CardContent>
    </Card>
  )
}

export function MitreGroupsView({ rules, isLoading }: MitreGroupsViewProps) {
  const groups = useMemo(() => {
    const map = new Map<string, Rule[]>()

    for (const rule of rules) {
      const tactics = parseStringArray(rule.mitre_tactics)
      if (tactics.length > 0) {
        for (const tactic of tactics) {
          const list = map.get(tactic) ?? []
          list.push(rule)
          map.set(tactic, list)
        }
      } else {
        const list = map.get('__unmapped__') ?? []
        list.push(rule)
        map.set('__unmapped__', list)
      }
    }

    const ordered: TacticGroup[] = []
    for (const [id, name] of Object.entries(MITRE_TACTICS)) {
      const groupRules = map.get(id)
      if (groupRules && groupRules.length > 0) {
        ordered.push({
          tactic: id,
          name,
          rules: groupRules.sort((a, b) => a.name.localeCompare(b.name)),
        })
      }
    }

    const unmapped = map.get('__unmapped__')
    if (unmapped && unmapped.length > 0) {
      ordered.push({
        tactic: '—',
        name: 'Unmapped',
        rules: unmapped.sort((a, b) => a.name.localeCompare(b.name)),
      })
    }

    return ordered
  }, [rules])

  if (isLoading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>
  }

  if (groups.length === 0) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          No MITRE groups available.
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        Rules grouped by MITRE ATT&CK tactic. Expand a group to see its rules.
      </p>
      {groups.map((group) => (
        <TacticCard key={group.tactic + group.name} group={group} />
      ))}
    </div>
  )
}
