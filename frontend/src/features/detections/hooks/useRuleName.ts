import { useQuery } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { ApiResponse, Rule } from '@/shared/types'

export interface RuleLookup {
  byId: (id: string | null | undefined) => Rule | undefined
  ruleName: (id: string | null | undefined) => string
}

export function useRuleName(): RuleLookup {
  const { data } = useQuery<ApiResponse<Rule[]>>({
    queryKey: ['rules'],
    queryFn: () => api.get(`/rules?page=1&size=1000`).then((res) => res.data),
  })

  const rules = data?.data ?? []

  const byId = (id: string | null | undefined) =>
    rules.find((r) => r.id === id)

  const ruleName = (id: string | null | undefined) => {
    if (!id) return 'Unknown rule'
    return byId(id)?.name ?? id.slice(0, 8)
  }

  return { byId, ruleName }
}
