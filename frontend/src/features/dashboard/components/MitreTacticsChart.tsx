import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'
import { tacticName } from '@/shared/lib/mitre'

interface MitreTactic {
  tactic: string
  count: number
}

interface MitreTacticsChartProps {
  data: MitreTactic[]
}

export function MitreTacticsChart({ data }: MitreTacticsChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        No MITRE data available
      </div>
    )
  }

  const formattedData = data.map((item) => ({
    name: `${tacticName(item.tactic)} (${item.tactic})`,
    count: item.count,
  }))

  return (
    <ResponsiveContainer width="100%" height={250}>
      <BarChart data={formattedData} layout="vertical">
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis type="number" tick={{ fontSize: 12 }} />
        <YAxis type="category" dataKey="name" width={170} tick={{ fontSize: 11 }} />
        <Tooltip />
        <Bar dataKey="count" fill="#3b82f6" radius={[0, 4, 4, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}
