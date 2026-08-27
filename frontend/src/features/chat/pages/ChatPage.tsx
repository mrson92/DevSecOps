import { useState, useRef, useEffect } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import api from '@/shared/lib/api'
import type { AiAgent, Persona, ApiResponse } from '@/shared/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

interface Message {
  id: string
  role: 'user' | 'assistant'
  content: string
  agentName?: string
  model?: string
  tokensUsed?: number
  timestamp: Date
}

export function ChatPage() {
  const [selectedAgentId, setSelectedAgentId] = useState<string>('')
  const [messages, setMessages] = useState<Message[]>([])
  const [inputValue, setInputValue] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)

  const { data: agentsData } = useQuery<ApiResponse<AiAgent[]>>({
    queryKey: ['agents'],
    queryFn: () => api.get('/agents').then((res) => res.data),
  })

  const { data: personasData } = useQuery<ApiResponse<Persona[]>>({
    queryKey: ['personas'],
    queryFn: () => api.get('/personas').then((res) => res.data),
  })

  const chatMutation = useMutation({
    mutationFn: (data: { agent_id: string; message: string }) =>
      api.post('/chat', data).then((res) => res.data.data),
    onSuccess: (data) => {
      const assistantMessage: Message = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: data.reply,
        agentName: data.agent_name,
        model: data.model,
        tokensUsed: data.tokens_used,
        timestamp: new Date(),
      }
      setMessages((prev) => [...prev, assistantMessage])
    },
  })

  const agents = agentsData?.data ?? []
  const personas = personasData?.data ?? []
  const enabledAgents = agents.filter((a) => a.enabled)

  const getPersonaName = (personaId: string) => {
    const persona = personas.find((p) => p.id === personaId)
    return persona?.name ?? 'Unknown'
  }

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  const handleSend = () => {
    if (!inputValue.trim() || !selectedAgentId) return

    const userMessage: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content: inputValue.trim(),
      timestamp: new Date(),
    }
    setMessages((prev) => [...prev, userMessage])

    chatMutation.mutate({
      agent_id: selectedAgentId,
      message: inputValue.trim(),
    })

    setInputValue('')
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const clearChat = () => {
    setMessages([])
  }

  return (
    <div className="flex h-[calc(100vh-4rem)] gap-4">
      <div className="w-64 flex-shrink-0">
        <Card className="h-full">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">Select Agent</CardTitle>
          </CardHeader>
          <CardContent className="p-2">
            {enabledAgents.length === 0 ? (
              <p className="text-xs text-muted-foreground p-2">
                No active agents. Create one first.
              </p>
            ) : (
              <div className="space-y-1">
                {enabledAgents.map((agent) => (
                  <button
                    key={agent.id}
                    onClick={() => setSelectedAgentId(agent.id)}
                    className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
                      selectedAgentId === agent.id
                        ? 'bg-primary text-primary-foreground'
                        : 'hover:bg-muted'
                    }`}
                  >
                    <div className="font-medium">{agent.name}</div>
                    <div className="text-xs opacity-70">
                      {getPersonaName(agent.persona_id)}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="flex-1 flex flex-col">
        <Card className="flex-1 flex flex-col">
          <CardHeader className="flex flex-row items-center justify-between pb-3">
            <CardTitle>
              {selectedAgentId
                ? `Chat with ${enabledAgents.find((a) => a.id === selectedAgentId)?.name ?? 'Agent'}`
                : 'Select an agent to start'}
            </CardTitle>
            {messages.length > 0 && (
              <Button variant="outline" size="sm" onClick={clearChat}>
                Clear Chat
              </Button>
            )}
          </CardHeader>
          <CardContent className="flex-1 overflow-y-auto p-4 space-y-4">
            {messages.length === 0 ? (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                {selectedAgentId
                  ? 'Type a message to start the conversation'
                  : 'Select an agent from the sidebar'}
              </div>
            ) : (
              messages.map((msg) => (
                <div
                  key={msg.id}
                  className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
                >
                  <div
                    className={`max-w-[70%] rounded-lg p-3 ${
                      msg.role === 'user'
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted'
                    }`}
                  >
                    {msg.role === 'assistant' && (
                      <div className="text-xs font-medium mb-1 opacity-70">
                        {msg.agentName} ({msg.model})
                      </div>
                    )}
                    <div className="whitespace-pre-wrap text-sm">{msg.content}</div>
                    <div className="text-xs opacity-50 mt-1">
                      {msg.timestamp.toLocaleTimeString()}
                      {msg.tokensUsed && ` | ${msg.tokensUsed} tokens`}
                    </div>
                  </div>
                </div>
              ))
            )}
            <div ref={messagesEndRef} />
          </CardContent>
          <div className="p-4 border-t">
            <div className="flex gap-2">
              <textarea
                value={inputValue}
                onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setInputValue(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={selectedAgentId ? 'Type your message...' : 'Select an agent first'}
                disabled={!selectedAgentId}
                rows={2}
                className="flex-1 px-3 py-2 rounded-md border border-border bg-background text-sm resize-none disabled:opacity-50"
              />
              <Button
                onClick={handleSend}
                disabled={!inputValue.trim() || !selectedAgentId || chatMutation.isPending}
              >
                {chatMutation.isPending ? 'Sending...' : 'Send'}
              </Button>
            </div>
          </div>
        </Card>
      </div>
    </div>
  )
}
