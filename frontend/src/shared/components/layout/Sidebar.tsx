import { useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { cn } from '@/lib/utils'

const navigation = [
  { name: 'Dashboard', href: '/', icon: '📊' },
  { name: 'Rules', href: '/rules', icon: '📋' },
  { name: 'Detections', href: '/detections', icon: '🔍' },
  { name: 'Logs', href: '/logs', icon: '📜' },
  { name: 'AI Agents', href: '/agents', icon: '🤖' },
  { name: 'Chat', href: '/chat', icon: '💬' },
  { name: 'Reports', href: '/reports', icon: '📈' },
  { name: 'Settings', href: '/settings', icon: '⚙️' },
]

const COLLAPSED_KEY = 'aads-sidebar-collapsed'

function loadCollapsed(): boolean {
  try {
    return localStorage.getItem(COLLAPSED_KEY) === '1'
  } catch {
    return false
  }
}

function saveCollapsed(collapsed: boolean) {
  try {
    localStorage.setItem(COLLAPSED_KEY, collapsed ? '1' : '0')
  } catch {
    /* ignore */
  }
}

export function Sidebar() {
  const location = useLocation()
  const [collapsed, setCollapsed] = useState(loadCollapsed)

  const toggle = () => {
    setCollapsed((prev) => {
      saveCollapsed(!prev)
      return !prev
    })
  }

  return (
    <aside
      className={cn(
        'bg-card border-r border-border min-h-screen transition-all duration-200',
        collapsed ? 'w-16' : 'w-64'
      )}
    >
      <div className={cn('p-4 flex items-center', collapsed ? 'justify-center' : 'justify-between gap-2')}>
        {!collapsed && (
          <div className="min-w-0">
            <h1 className="text-xl font-bold">AADS</h1>
            <p className="text-sm text-muted-foreground truncate">Abnormal Access Detection</p>
          </div>
        )}
        <button
          onClick={toggle}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          className="shrink-0 p-2 rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
        >
          <span aria-hidden>{collapsed ? '▶' : '◀'}</span>
        </button>
      </div>
      <nav className={cn('pb-4', collapsed ? 'px-2' : 'px-4')}>
        <ul className="space-y-1">
          {navigation.map((item) => (
            <li key={item.name}>
              <Link
                to={item.href}
                className={cn(
                  'group relative flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors',
                  collapsed && 'justify-center px-0',
                  location.pathname === item.href
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                )}
              >
                <span>{item.icon}</span>
                {!collapsed && <span>{item.name}</span>}
                {collapsed && (
                  <span className="pointer-events-none absolute left-full ml-2 top-1/2 -translate-y-1/2 z-50 whitespace-nowrap rounded-md border border-border bg-popover text-popover-foreground px-2 py-1 text-xs shadow-md opacity-0 transition-opacity group-hover:opacity-100">
                    {item.name}
                  </span>
                )}
              </Link>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  )
}