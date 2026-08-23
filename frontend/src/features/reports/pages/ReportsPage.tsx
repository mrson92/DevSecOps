import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function ReportsPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Reports</h1>
        <Button>Generate Report</Button>
      </div>

      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          No reports generated yet. Click "Generate Report" to create your first report.
        </CardContent>
      </Card>
    </div>
  )
}
