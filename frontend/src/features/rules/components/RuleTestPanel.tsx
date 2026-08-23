import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

interface RuleTestPanelProps {
  ruleId: string
}

interface TestResult {
  id: string
  matched_count: number
  matched_logs: Array<Record<string, unknown>>
  execution_time_ms: number
  status: string
}

export function RuleTestPanel({ ruleId }: RuleTestPanelProps) {
  const [result, setResult] = useState<TestResult | null>(null)

  const testMutation = useMutation({
    mutationFn: () =>
      api.post(`/rules/${ruleId}/test`, { test_type: 'dry_run' }).then((res) => res.data.data),
    onSuccess: (data: TestResult) => setResult(data),
  })

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Rule Test</CardTitle>
        <Button
          variant="outline"
          size="sm"
          onClick={() => testMutation.mutate()}
          disabled={testMutation.isPending}
        >
          {testMutation.isPending ? 'Testing...' : 'Run Test'}
        </Button>
      </CardHeader>
      <CardContent>
        {testMutation.isError && (
          <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
            Test failed: {(testMutation.error as Error).message}
          </div>
        )}

        {result && (
          <div className="space-y-3">
            <div className="grid grid-cols-3 gap-4 text-sm">
              <div>
                <span className="text-muted-foreground">Status</span>
                <div className="font-medium">{result.status}</div>
              </div>
              <div>
                <span className="text-muted-foreground">Matched</span>
                <div className="font-medium">{result.matched_count} entries</div>
              </div>
              <div>
                <span className="text-muted-foreground">Execution Time</span>
                <div className="font-medium">{result.execution_time_ms}ms</div>
              </div>
            </div>
            {result.matched_logs.length > 0 && (
              <div>
                <span className="text-sm text-muted-foreground">Matched Logs (first 5)</span>
                <pre className="mt-1 p-3 bg-muted rounded-md overflow-x-auto text-xs max-h-64 overflow-y-auto">
                  {JSON.stringify(result.matched_logs.slice(0, 5), null, 2)}
                </pre>
              </div>
            )}
          </div>
        )}

        {!result && !testMutation.isPending && !testMutation.isError && (
          <p className="text-sm text-muted-foreground">
            Click "Run Test" to test this rule against current logs.
          </p>
        )}
      </CardContent>
    </Card>
  )
}
