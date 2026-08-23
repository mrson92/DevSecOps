import { PieChart, Pie, Cell, ResponsiveContainer, Legend, Tooltip } from 'recharts'

interface SeverityChartProps {
  critical: number
  high: number
  medium: number
  low: number
}

const COLORS = ['#ef4444', '#f97316', '#eab308', '#22c55e']

export function SeverityChart({ critical, high, medium, low }: SeverityChartProps) {
  const data = [
    { name: 'Critical', value: critical },
    { name: 'High', value: high },
    { name: 'Medium', value: medium },
    { name: 'Low', value: low },
  ].filter((item) => item.value > 0)

  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        No detections
      </div>
    )
  }

  return (
    <ResponsiveContainer width="100%" height={250}>
      <PieChart>
        <Pie
          data={data}
          cx="50%"
          cy="50%"
          innerRadius={60}
          outerRadius={90}
          paddingAngle={2}
          dataKey="value"
          label={({ name, value }) => `${name}: ${value}`}
        >
          {data.map((_, index) => (
            <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
          ))}
        </Pie>
        <Tooltip />
        <Legend />
      </PieChart>
    </ResponsiveContainer>
  )
}
