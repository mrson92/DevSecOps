import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { DataSource, NotificationChannel, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function SettingsPage() {
  const queryClient = useQueryClient()
  const [showAddSource, setShowAddSource] = useState(false)
  const [showAddChannel, setShowAddChannel] = useState(false)
  const [sourceForm, setSourceForm] = useState({ name: '', type: 'elasticsearch', config: '', target: '' })
  const [channelForm, setChannelForm] = useState({ name: '', type: 'webhook', config: '' })

  const { data: sourcesData } = useQuery<ApiResponse<DataSource[]>>({
    queryKey: ['data-sources'],
    queryFn: () => api.get('/data-sources').then((res) => res.data),
  })

  const { data: channelsData } = useQuery<ApiResponse<NotificationChannel[]>>({
    queryKey: ['notification-channels'],
    queryFn: () => api.get('/notifications/channels').then((res) => res.data),
  })

  const createSourceMutation = useMutation({
    mutationFn: (data: typeof sourceForm) => api.post('/data-sources', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['data-sources'] })
      setShowAddSource(false)
      setSourceForm({ name: '', type: 'elasticsearch', config: '', target: '' })
    },
  })

  const deleteSourceMutation = useMutation({
    mutationFn: (id: string) => api.delete(`/data-sources/${id}`),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['data-sources'] }),
  })

  const testSourceMutation = useMutation({
    mutationFn: (id: string) => api.post(`/data-sources/${id}/test`),
  })

  const createChannelMutation = useMutation({
    mutationFn: (data: typeof channelForm) => api.post('/notifications/channels', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['notification-channels'] })
      setShowAddChannel(false)
      setChannelForm({ name: '', type: 'webhook', config: '' })
    },
  })

  const deleteChannelMutation = useMutation({
    mutationFn: (id: string) => api.delete(`/notifications/channels/${id}`),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['notification-channels'] }),
  })

  const testChannelMutation = useMutation({
    mutationFn: (id: string) => api.post(`/notifications/channels/${id}/test`),
  })

  const sources = sourcesData?.data ?? []
  const channels = channelsData?.data ?? []

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Data Sources</CardTitle>
          <Button variant="outline" size="sm" onClick={() => setShowAddSource(!showAddSource)}>
            {showAddSource ? 'Cancel' : 'Add Data Source'}
          </Button>
        </CardHeader>
        <CardContent className="space-y-4">
          {showAddSource && (
            <div className="grid grid-cols-2 gap-4 p-4 border rounded-md">
              <input
                type="text"
                placeholder="Name"
                value={sourceForm.name}
                onChange={(e) => setSourceForm({ ...sourceForm, name: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm"
              />
              <select
                value={sourceForm.type}
                onChange={(e) => setSourceForm({ ...sourceForm, type: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm"
              >
                <option value="elasticsearch">ElasticSearch</option>
                <option value="loki">Loki</option>
                <option value="postgresql">PostgreSQL</option>
              </select>
              <input
                type="text"
                placeholder='Config JSON (e.g. {"url": "http://localhost:9200"})'
                value={sourceForm.config}
                onChange={(e) => setSourceForm({ ...sourceForm, config: e.target.value })}
                className="col-span-2 px-3 py-2 rounded-md border border-border bg-background text-sm font-mono"
              />
              <input
                type="text"
                placeholder="Target (e.g. logs-*)"
                value={sourceForm.target}
                onChange={(e) => setSourceForm({ ...sourceForm, target: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm"
              />
              <Button
                onClick={() => createSourceMutation.mutate(sourceForm)}
                disabled={!sourceForm.name || createSourceMutation.isPending}
              >
                Create
              </Button>
            </div>
          )}
          {sources.length === 0 ? (
            <p className="text-sm text-muted-foreground">No data sources configured.</p>
          ) : (
            sources.map((source) => (
              <div key={source.id} className="flex items-center justify-between p-3 border rounded-md">
                <div>
                  <div className="font-medium">{source.name}</div>
                  <div className="text-sm text-muted-foreground">{source.type} - {source.target}</div>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => testSourceMutation.mutate(source.id)}
                    disabled={testSourceMutation.isPending}
                  >
                    Test
                  </Button>
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={() => deleteSourceMutation.mutate(source.id)}
                  >
                    Delete
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Notification Channels</CardTitle>
          <Button variant="outline" size="sm" onClick={() => setShowAddChannel(!showAddChannel)}>
            {showAddChannel ? 'Cancel' : 'Add Channel'}
          </Button>
        </CardHeader>
        <CardContent className="space-y-4">
          {showAddChannel && (
            <div className="grid grid-cols-2 gap-4 p-4 border rounded-md">
              <input
                type="text"
                placeholder="Name"
                value={channelForm.name}
                onChange={(e) => setChannelForm({ ...channelForm, name: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm"
              />
              <select
                value={channelForm.type}
                onChange={(e) => setChannelForm({ ...channelForm, type: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm"
              >
                <option value="webhook">Webhook</option>
                <option value="email">Email</option>
                <option value="dashboard">Dashboard</option>
              </select>
              <input
                type="text"
                placeholder='Config JSON (e.g. {"url": "https://hooks.slack.com/..."})'
                value={channelForm.config}
                onChange={(e) => setChannelForm({ ...channelForm, config: e.target.value })}
                className="col-span-2 px-3 py-2 rounded-md border border-border bg-background text-sm font-mono"
              />
              <Button
                onClick={() => createChannelMutation.mutate(channelForm)}
                disabled={!channelForm.name || createChannelMutation.isPending}
              >
                Create
              </Button>
            </div>
          )}
          {channels.length === 0 ? (
            <p className="text-sm text-muted-foreground">No notification channels configured.</p>
          ) : (
            channels.map((channel) => (
              <div key={channel.id} className="flex items-center justify-between p-3 border rounded-md">
                <div>
                  <div className="font-medium">{channel.name}</div>
                  <div className="text-sm text-muted-foreground">{channel.type}</div>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => testChannelMutation.mutate(channel.id)}
                    disabled={testChannelMutation.isPending}
                  >
                    Test
                  </Button>
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={() => deleteChannelMutation.mutate(channel.id)}
                  >
                    Delete
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>System Information</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div><span className="text-muted-foreground">Version:</span> 0.3.0</div>
            <div><span className="text-muted-foreground">Environment:</span> Development</div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
