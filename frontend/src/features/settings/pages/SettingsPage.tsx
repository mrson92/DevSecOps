import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { DataSource, NotificationChannel, OidcSettings, OidcTestResult, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

interface EsConfig {
  url: string
  username: string
  password: string
  index_prefix: string
}

interface LokiConfig {
  url: string
  tenant_id: string
}

interface PostgresConfig {
  host: string
  port: string
  database: string
  user: string
  password: string
  ssl_mode: string
}

const DEFAULT_ES_CONFIG: EsConfig = { url: 'http://localhost:9200', username: '', password: '', index_prefix: 'aads' }
const DEFAULT_LOKI_CONFIG: LokiConfig = { url: 'http://localhost:3100', tenant_id: '' }
const DEFAULT_PG_CONFIG: PostgresConfig = { host: 'localhost', port: '5432', database: 'postgres', user: 'postgres', password: '', ssl_mode: 'prefer' }

function buildConfig(_type: string, fields: EsConfig | LokiConfig | PostgresConfig): string {
  const cleaned: Record<string, string> = {}
  for (const [k, v] of Object.entries(fields)) {
    if (v !== '') cleaned[k] = v
  }
  return JSON.stringify(cleaned)
}

function parseConfig(type: string, configStr: string): EsConfig | LokiConfig | PostgresConfig {
  try {
    const parsed = JSON.parse(configStr)
    if (type === 'elasticsearch') {
      return { url: parsed.url || '', username: parsed.username || '', password: parsed.password || '', index_prefix: parsed.index_prefix || 'aads' }
    }
    if (type === 'loki') {
      return { url: parsed.url || '', tenant_id: parsed.tenant_id || '' }
    }
    if (type === 'postgresql') {
      return { host: parsed.host || '', port: String(parsed.port || '5432'), database: parsed.database || '', user: parsed.user || '', password: parsed.password || '', ssl_mode: parsed.ssl_mode || 'prefer' }
    }
  } catch { /* ignore */ }
  if (type === 'elasticsearch') return { ...DEFAULT_ES_CONFIG }
  if (type === 'loki') return { ...DEFAULT_LOKI_CONFIG }
  return { ...DEFAULT_PG_CONFIG }
}

function EsFields({ config, onChange }: { config: EsConfig; onChange: (c: EsConfig) => void }) {
  return (
    <>
      <div className="space-y-2">
        <label className="text-sm font-medium">URL *</label>
        <input type="text" placeholder="http://localhost:9200" value={config.url}
          onChange={(e) => onChange({ ...config, url: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Username</label>
        <input type="text" placeholder="elastic" value={config.username}
          onChange={(e) => onChange({ ...config, username: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Password</label>
        <input type="password" placeholder="••••••••" value={config.password}
          onChange={(e) => onChange({ ...config, password: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Index Prefix</label>
        <input type="text" placeholder="aads" value={config.index_prefix}
          onChange={(e) => onChange({ ...config, index_prefix: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
    </>
  )
}

function LokiFields({ config, onChange }: { config: LokiConfig; onChange: (c: LokiConfig) => void }) {
  return (
    <>
      <div className="space-y-2">
        <label className="text-sm font-medium">URL *</label>
        <input type="text" placeholder="http://localhost:3100" value={config.url}
          onChange={(e) => onChange({ ...config, url: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Tenant ID</label>
        <input type="text" placeholder="Optional" value={config.tenant_id}
          onChange={(e) => onChange({ ...config, tenant_id: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
    </>
  )
}

function PostgresFields({ config, onChange }: { config: PostgresConfig; onChange: (c: PostgresConfig) => void }) {
  return (
    <>
      <div className="space-y-2">
        <label className="text-sm font-medium">Host *</label>
        <input type="text" placeholder="localhost" value={config.host}
          onChange={(e) => onChange({ ...config, host: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Port</label>
        <input type="text" placeholder="5432" value={config.port}
          onChange={(e) => onChange({ ...config, port: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Database *</label>
        <input type="text" placeholder="postgres" value={config.database}
          onChange={(e) => onChange({ ...config, database: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">User *</label>
        <input type="text" placeholder="postgres" value={config.user}
          onChange={(e) => onChange({ ...config, user: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Password</label>
        <input type="password" placeholder="••••••••" value={config.password}
          onChange={(e) => onChange({ ...config, password: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">SSL Mode</label>
        <select value={config.ssl_mode}
          onChange={(e) => onChange({ ...config, ssl_mode: e.target.value })}
          className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm">
          <option value="disable">Disable</option>
          <option value="prefer">Prefer</option>
          <option value="require">Require</option>
          <option value="verify-ca">Verify CA</option>
          <option value="verify-full">Verify Full</option>
        </select>
      </div>
    </>
  )
}

interface SourceFormProps {
  initialName?: string
  initialType?: string
  initialConfig?: string
  initialTarget?: string
  initialEnabled?: boolean
  initialPrimary?: boolean
  submitLabel: string
  onSubmit: (data: { name: string; type: string; config: string; target: string; enabled: boolean; is_primary: boolean }) => void
  onCancel: () => void
  isPending?: boolean
}

function SourceForm({ initialName = '', initialType = 'elasticsearch', initialConfig = '', initialTarget = '', initialEnabled = true, initialPrimary = false, submitLabel, onSubmit, onCancel, isPending }: SourceFormProps) {
  const [name, setName] = useState(initialName)
  const [type, setType] = useState(initialType)
  const [target, setTarget] = useState(initialTarget)
  const [enabled, setEnabled] = useState(initialEnabled)
  const [isPrimary, setIsPrimary] = useState(initialPrimary)
  const [esConfig, setEsConfig] = useState<EsConfig>(() => parseConfig('elasticsearch', initialConfig) as EsConfig)
  const [lokiConfig, setLokiConfig] = useState<LokiConfig>(() => parseConfig('loki', initialConfig) as LokiConfig)
  const [pgConfig, setPgConfig] = useState<PostgresConfig>(() => parseConfig('postgresql', initialConfig) as PostgresConfig)

  useEffect(() => {
    if (!initialConfig) {
      if (type === 'elasticsearch') setEsConfig({ ...DEFAULT_ES_CONFIG })
      else if (type === 'loki') setLokiConfig({ ...DEFAULT_LOKI_CONFIG })
      else setPgConfig({ ...DEFAULT_PG_CONFIG })
    }
  }, [type])

  const handleTypeChange = (newType: string) => {
    setType(newType)
    if (newType === 'elasticsearch') setEsConfig(parseConfig('elasticsearch', initialConfig) as EsConfig)
    else if (newType === 'loki') setLokiConfig(parseConfig('loki', initialConfig) as LokiConfig)
    else setPgConfig(parseConfig('postgresql', initialConfig) as PostgresConfig)
  }

  const handleSubmit = () => {
    let config: string
    if (type === 'elasticsearch') config = buildConfig(type, esConfig)
    else if (type === 'loki') config = buildConfig(type, lokiConfig)
    else config = buildConfig(type, pgConfig)
    onSubmit({ name, type, config, target, enabled, is_primary: isPrimary })
  }

  const configFields = type === 'elasticsearch'
    ? <EsFields config={esConfig} onChange={setEsConfig} />
    : type === 'loki'
    ? <LokiFields config={lokiConfig} onChange={setLokiConfig} />
    : <PostgresFields config={pgConfig} onChange={setPgConfig} />

  return (
    <div className="p-4 border rounded-md space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <label className="text-sm font-medium">Name *</label>
          <input type="text" placeholder="My Elasticsearch" value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium">Type</label>
          <select value={type} onChange={(e) => handleTypeChange(e.target.value)}
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm">
            <option value="elasticsearch">ElasticSearch</option>
            <option value="loki">Loki</option>
            <option value="postgresql">PostgreSQL</option>
          </select>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        {configFields}
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <label className="text-sm font-medium">Target</label>
          <input type="text" placeholder="logs-*" value={target}
            onChange={(e) => setTarget(e.target.value)}
            className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
        </div>
        <div className="flex items-end gap-4">
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} className="rounded" />
            Enabled
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={isPrimary} onChange={(e) => setIsPrimary(e.target.checked)} className="rounded" />
            Primary
          </label>
        </div>
      </div>
      <div className="flex gap-2">
        <Button onClick={handleSubmit} disabled={!name || isPending}>
          {isPending ? 'Saving...' : submitLabel}
        </Button>
        <Button variant="outline" onClick={onCancel}>Cancel</Button>
      </div>
    </div>
  )
}

export function SettingsPage() {
  const queryClient = useQueryClient()
  const [showAddSource, setShowAddSource] = useState(false)
  const [editingSource, setEditingSource] = useState<DataSource | null>(null)
  const [showAddChannel, setShowAddChannel] = useState(false)
  const [channelForm, setChannelForm] = useState({ name: '', type: 'webhook', config: '' })
  const [oidcForm, setOidcForm] = useState<OidcSettings>({
    issuer_url: '', realm: '', client_id: '', client_secret: '', redirect_url: '', jwt_secret: '',
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
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['oidc-settings'] }),
  })

  const testOidcMutation = useMutation({
    mutationFn: () => api.post('/settings/oidc/test').then((res) => res.data.data as OidcTestResult),
  })

  const createSourceMutation = useMutation({
    mutationFn: (data: { name: string; type: string; config: string; target: string; enabled: boolean; is_primary: boolean }) =>
      api.post('/data-sources', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['data-sources'] })
      setShowAddSource(false)
    },
  })

  const updateSourceMutation = useMutation({
    mutationFn: ({ id, ...data }: { id: string; name: string; type: string; config: string; target: string; enabled: boolean; is_primary: boolean }) =>
      api.put(`/data-sources/${id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['data-sources'] })
      setEditingSource(null)
    },
  })

  const deleteSourceMutation = useMutation({
    mutationFn: (id: string) => api.delete(`/data-sources/${id}`),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['data-sources'] }),
  })

  const testSourceMutation = useMutation({
    mutationFn: (id: string) => api.post(`/data-sources/${id}/test`).then((res) => res.data),
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
    if (hasChanges) updateOidcMutation.mutate(oidcForm)
  }

  const testResult = testSourceMutation.data
  const testResultData = testResult?.data

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      {/* Data Sources */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Data Sources</CardTitle>
          <Button variant="outline" size="sm" onClick={() => { setShowAddSource(!showAddSource); setEditingSource(null) }}>
            {showAddSource ? 'Cancel' : 'Add Data Source'}
          </Button>
        </CardHeader>
        <CardContent className="space-y-4">
          {showAddSource && (
            <SourceForm
              submitLabel="Create"
              onSubmit={(data) => createSourceMutation.mutate(data)}
              onCancel={() => setShowAddSource(false)}
              isPending={createSourceMutation.isPending}
            />
          )}
          {editingSource && (
            <SourceForm
              initialName={editingSource.name}
              initialType={editingSource.type}
              initialConfig={editingSource.config}
              initialTarget={editingSource.target}
              initialEnabled={editingSource.enabled}
              initialPrimary={editingSource.is_primary}
              submitLabel="Update"
              onSubmit={(data) => updateSourceMutation.mutate({ id: editingSource.id, ...data })}
              onCancel={() => setEditingSource(null)}
              isPending={updateSourceMutation.isPending}
            />
          )}

          {testSourceMutation.isSuccess && testResultData && (
            <div className={`p-3 rounded-md text-sm ${testResultData.status === 'connected'
              ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400'
              : 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400'}`}>
              {testResultData.message}
            </div>
          )}
          {testSourceMutation.isError && (
            <div className="p-3 rounded-md text-sm bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400">
              Connection test failed.
            </div>
          )}

          {sources.length === 0 && !showAddSource && !editingSource ? (
            <p className="text-sm text-muted-foreground">No data sources configured.</p>
          ) : (
            sources.map((source) => (
              <div key={source.id} className="flex items-center justify-between p-3 border rounded-md">
                <div>
                  <div className="font-medium">{source.name} {source.is_primary && <span className="text-xs bg-blue-100 text-blue-800 px-1.5 py-0.5 rounded ml-1">primary</span>}</div>
                  <div className="text-sm text-muted-foreground">{source.type} - {source.target || 'no target'}</div>
                </div>
                <div className="flex gap-2">
                  <Button variant="outline" size="sm"
                    onClick={() => testSourceMutation.mutate(source.id)}
                    disabled={testSourceMutation.isPending}>
                    Test
                  </Button>
                  <Button variant="outline" size="sm"
                    onClick={() => { setEditingSource(source); setShowAddSource(false) }}>
                    Edit
                  </Button>
                  <Button variant="destructive" size="sm"
                    onClick={() => deleteSourceMutation.mutate(source.id)}>
                    Delete
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      {/* Notification Channels */}
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
              <input type="text" placeholder="Name" value={channelForm.name}
                onChange={(e) => setChannelForm({ ...channelForm, name: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm" />
              <select value={channelForm.type}
                onChange={(e) => setChannelForm({ ...channelForm, type: e.target.value })}
                className="px-3 py-2 rounded-md border border-border bg-background text-sm">
                <option value="webhook">Webhook</option>
                <option value="email">Email</option>
                <option value="dashboard">Dashboard</option>
              </select>
              <input type="text" placeholder='Config JSON (e.g. {"url": "https://hooks.slack.com/..."})'
                value={channelForm.config}
                onChange={(e) => setChannelForm({ ...channelForm, config: e.target.value })}
                className="col-span-2 px-3 py-2 rounded-md border border-border bg-background text-sm font-mono" />
              <Button onClick={() => createChannelMutation.mutate(channelForm)}
                disabled={!channelForm.name || createChannelMutation.isPending}>
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
                  <Button variant="outline" size="sm"
                    onClick={() => testChannelMutation.mutate(channel.id)}
                    disabled={testChannelMutation.isPending}>
                    Test
                  </Button>
                  <Button variant="destructive" size="sm"
                    onClick={() => deleteChannelMutation.mutate(channel.id)}>
                    Delete
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      {/* OIDC Settings */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>OpenID Connect (OIDC) Settings</CardTitle>
          <div className="flex gap-2">
            <Button variant="outline" size="sm"
              onClick={() => testOidcMutation.mutate()}
              disabled={testOidcMutation.isPending}>
              {testOidcMutation.isPending ? 'Testing...' : 'Test Connection'}
            </Button>
            <Button size="sm" onClick={handleSaveOidc} disabled={updateOidcMutation.isPending}>
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
                <input type="text" placeholder="http://localhost:8080/realms/master"
                  value={oidcForm.issuer_url || oidcSettings?.issuer_url || ''}
                  onChange={(e) => handleOidcChange('issuer_url', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Realm</label>
                <input type="text" placeholder="master"
                  value={oidcForm.realm || oidcSettings?.realm || ''}
                  onChange={(e) => handleOidcChange('realm', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Client ID</label>
                <input type="text" placeholder="aads"
                  value={oidcForm.client_id || oidcSettings?.client_id || ''}
                  onChange={(e) => handleOidcChange('client_id', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Client Secret</label>
                <div className="relative">
                  <input type={showSecrets ? 'text' : 'password'} placeholder="••••••••"
                    value={oidcForm.client_secret || oidcSettings?.client_secret || ''}
                    onChange={(e) => handleOidcChange('client_secret', e.target.value)}
                    className="w-full px-3 py-2 pr-10 rounded-md border border-border bg-background text-sm" />
                  <button type="button" onClick={() => setShowSecrets(!showSecrets)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground">
                    {showSecrets ? '🙈' : '👁️'}
                  </button>
                </div>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Redirect URL</label>
                <input type="text" placeholder="http://localhost:3000/auth/callback"
                  value={oidcForm.redirect_url || oidcSettings?.redirect_url || ''}
                  onChange={(e) => handleOidcChange('redirect_url', e.target.value)}
                  className="w-full px-3 py-2 rounded-md border border-border bg-background text-sm" />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">JWT Secret</label>
                <div className="relative">
                  <input type={showSecrets ? 'text' : 'password'} placeholder="••••••••"
                    value={oidcForm.jwt_secret || oidcSettings?.jwt_secret || ''}
                    onChange={(e) => handleOidcChange('jwt_secret', e.target.value)}
                    className="w-full px-3 py-2 pr-10 rounded-md border border-border bg-background text-sm" />
                  <button type="button" onClick={() => setShowSecrets(!showSecrets)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground">
                    {showSecrets ? '🙈' : '👁️'}
                  </button>
                </div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* System Information */}
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
