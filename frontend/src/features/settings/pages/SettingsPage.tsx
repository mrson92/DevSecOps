import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { DataSource, NotificationChannel, OidcSettings, OidcTestResult, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function SettingsPage() {
  const queryClient = useQueryClient()
  const [showAddSource, setShowAddSource] = useState(false)
  const [showAddChannel, setShowAddChannel] = useState(false)
  const [sourceForm, setSourceForm] = useState({ name: '', type: 'elasticsearch', config: '', target: '' })
  const [channelForm, setChannelForm] = useState({ name: '', type: 'webhook', config: '' })
  const [oidcForm, setOidcForm] = useState<OidcSettings>({
    issuer_url: '',
    realm: '',
    client_id: '',
    client_secret: '',
    redirect_url: '',
    jwt_secret: '',
  })
  const [showSecrets, setShowSecrets] = useState(false)

  const { data: sourcesData } = useQuery<ApiResponse<DataSource[]>>({
    queryKey: ['data-sources'],
    queryFn: () => api.get('/data-sources').then((res) => res.data),
  })

  const { data: channelsData } = useQuery<ApiResponse<NotificationChannel[]>>({
    queryKey: ['notification-channels'],
    queryFn: () => api.get('/notifications/channels').then((res) => res.data),
  })

  const { data: oidcData, isLoading: oidcLoading } = useQuery<ApiResponse<OidcSettings>>({
    queryKey: ['oidc-settings'],
    queryFn: () => api.get('/settings/oidc').then((res) => res.data),
  })

  const updateOidcMutation = useMutation({
    mutationFn: (data: Partial<OidcSettings>) => api.put('/settings/oidc', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['oidc-settings'] })
    },
  })

  const testOidcMutation = useMutation({
    mutationFn: () => api.post('/settings/oidc/test').then((res) => res.data.data as OidcTestResult),
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
  const oidcSettings = oidcData?.data

  const handleOidcChange = (field: keyof OidcSettings, value: string) => {
    setOidcForm((prev) => ({ ...prev, [field]: value }))
  }

  const handleSaveOidc = () => {
    const hasChanges = Object.keys(oidcForm).some(
      (key) => oidcForm[key as keyof OidcSettings] !== (oidcSettings?.[key as keyof OidcSettings] ?? '')
    )
    if (hasChanges) {
      updateOidcMutation.mutate(oidcForm)
    }
  }

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
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>OpenID Connect (OIDC) Settings</CardTitle>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => testOidcMutation.mutate()}
              disabled={testOidcMutation.isPending}
            >
              {testOidcMutation.isPending ? 'Testing...' : 'Test Connection'}
            </Button>
            <Button
              size="sm"
              onClick={handleSaveOidc}
              disabled={updateOidcMutation.isPending}
            >
              {updateOidcMutation.isPending ? 'Saving...' : 'Save Changes'}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {testOidcMutation.isSuccess && testOidcMutation.data && (
            <div className={`p-3 rounded-md text-sm ${testOidcMutation.data.status === 'connected' ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400' : 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400'}`}>
              {testOidcMutation.data.message}
              {testOidcMutation.data.discovery && (
                <div className="mt-2 text-xs opacity-75">
                  <div>Token Endpoint: {testOidcMutation.data.discovery.token_endpoint}</div>
                  <div>Authorization Endpoint: {testOidcMutation.data.discovery.authorization_endpoint}</div>
                </div>
              )}
            </div>
          )}
          {testOidcMutation.isError && (
            <div className="p-3 rounded-md text-sm bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400">
              Connection test failed. Please check your settings.
            </div>
          )}

          {oidcLoading ? (
            <p className="text-sm text-muted-foreground">Loading OIDC settings...</p>
          ) : (
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Issuer URL</label>
                <input
                  type="text"
                  placeholder="http://localhost:8080/realms/master"
                  value={oidcForm.issuer_url || oidcSettings?.issuer_url || ''}
                  onChange={(e) => handleOidcChange('issuer_url', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">Realm</label>
                <input
                  type="text"
                  placeholder="master"
                  value={oidcForm.realm || oidcSettings?.realm || ''}
                  onChange={(e) => handleOidcChange('realm', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">Client ID</label>
                <input
                  type="text"
                  placeholder="aads"
                  value={oidcForm.client_id || oidcSettings?.client_id || ''}
                  onChange={(e) => handleOidcChange('client_id', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">Client Secret</label>
                <div className="relative">
                  <input
                    type={showSecrets ? 'text' : 'password'}
                    placeholder="••••••••"
                    value={oidcForm.client_secret || oidcSettings?.client_secret || ''}
                    onChange={(e) => handleOidcChange('client_secret', e.target.value)}
                    className="w-full px-3 py-2 pr-10 rounded-md border border-border bg-background text-sm"
                  />
                  <button
                    type="button"
                    onClick={() => setShowSecrets(!showSecrets)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  >
                    {showSecrets ? '🙈' : '👁️'}
                  </button>
                </div>
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">Redirect URL</label>
                <input
                  type="text"
                  placeholder="http://localhost:3000/auth/callback"
                  value={oidcForm.redirect_url || oidcSettings?.redirect_url || ''}
                  onChange={(e) => handleOidcChange('redirect_url', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">JWT Secret</label>
                <div className="relative">
                  <input
                    type={showSecrets ? 'text' : 'password'}
                    placeholder="••••••••"
                    value={oidcForm.jwt_secret || oidcSettings?.jwt_secret || ''}
                    onChange={(e) => handleOidcChange('jwt_secret', e.target.value)}
                    className="w-full px-3 py-2 pr-10 rounded-md border border-border bg-background text-sm"
                  />
                  <button
                    type="button"
                    onClick={() => setShowSecrets(!showSecrets)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  >
                    {showSecrets ? '🙈' : '👁️'}
                  </button>
                </div>
              </div>
            </div>
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
