import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

interface TopIp {
  ip: string
  count: number
}

interface TopIpsChartProps {
  data: TopIp[]
}

export function TopIpsChart({ data }: TopIpsChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        No data available
      </div>
    )
  }

  return (
    <ResponsiveContainer width="100%" height={250}>
      <BarChart data={data} layout="vertical">
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis type="number" tick={{ fontSize: 12 }} />
        <YAxis type="category" dataKey="ip" width={120} tick={{ fontSize: 11 }} />
        <Tooltip />
        <Bar dataKey="count" fill="#f59e0b" radius={[0, 4, 4, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}
