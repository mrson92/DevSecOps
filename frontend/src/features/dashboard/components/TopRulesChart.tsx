import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

interface TopRule {
  id: string
  name: string
  severity: string
  count: number
}

interface TopRulesChartProps {
  data: TopRule[]
}

export function TopRulesChart({ data }: TopRulesChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        No data available
      </div>
    )
  }

  const formattedData = data.map((item) => ({
    name: item.name.length > 20 ? item.name.slice(0, 20) + '...' : item.name,
    count: item.count,
    severity: item.severity,
  }))

  return (
    <ResponsiveContainer width="100%" height={250}>
      <BarChart data={formattedData} layout="vertical">
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis type="number" tick={{ fontSize: 12 }} />
        <YAxis type="category" dataKey="name" width={150} tick={{ fontSize: 11 }} />
        <Tooltip />
        <Bar dataKey="count" fill="#8b5cf6" radius={[0, 4, 4, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}
