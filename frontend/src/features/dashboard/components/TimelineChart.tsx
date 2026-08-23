import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

interface TimelineData {
  time: string
  count: number
}

interface TimelineChartProps {
  data: TimelineData[]
}

export function TimelineChart({ data }: TimelineChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        No data available
      </div>
    )
  }

  const formattedData = data.map((item) => ({
    ...item,
    time: new Date(item.time).toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' }),
  }))

  return (
    <ResponsiveContainer width="100%" height={250}>
      <BarChart data={formattedData}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis dataKey="time" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} />
        <Tooltip />
        <Bar dataKey="count" fill="#3b82f6" radius={[4, 4, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}
