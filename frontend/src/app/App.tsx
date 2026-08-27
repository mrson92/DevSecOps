import { Routes, Route } from 'react-router-dom'
import { Layout } from '@/shared/components/layout/Layout'
import { DashboardPage } from '@/features/dashboard/pages/DashboardPage'
import { RulesPage } from '@/features/rules/pages/RulesPage'
import { RuleDetailPage } from '@/features/rules/pages/RuleDetailPage'
import { DetectionsPage } from '@/features/detections/pages/DetectionsPage'
import { DetectionDetailPage } from '@/features/detections/pages/DetectionDetailPage'
import { AgentsPage } from '@/features/agents/pages/AgentsPage'
import { ChatPage } from '@/features/chat/pages/ChatPage'
import { ReportsPage } from '@/features/reports/pages/ReportsPage'
import { SettingsPage } from '@/features/settings/pages/SettingsPage'

export function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/rules" element={<RulesPage />} />
        <Route path="/rules/:id" element={<RuleDetailPage />} />
        <Route path="/detections" element={<DetectionsPage />} />
        <Route path="/detections/:id" element={<DetectionDetailPage />} />
        <Route path="/agents" element={<AgentsPage />} />
        <Route path="/chat" element={<ChatPage />} />
        <Route path="/reports" element={<ReportsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Routes>
    </Layout>
  )
}
