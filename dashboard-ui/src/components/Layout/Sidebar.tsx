import { Activity, BarChart3, Bot, FileSliders, RadioTower, Settings, WalletCards } from 'lucide-react'
import type { NavKey } from '../../types'

const items: Array<{ key: NavKey; label: string; icon: typeof BarChart3 }> = [
  { key: 'overview', label: 'Dashboard', icon: BarChart3 },
  { key: 'bets', label: 'Bets', icon: WalletCards },
  { key: 'strategies', label: 'Strategies', icon: FileSliders },
  { key: 'signals', label: 'Signals', icon: RadioTower },
  { key: 'config', label: 'Config', icon: Settings },
  { key: 'logs', label: 'Logs', icon: Activity }
]

type SidebarProps = {
  active: NavKey
  onNavigate: (key: NavKey) => void
}

export function Sidebar({ active, onNavigate }: SidebarProps) {
  return (
    <aside className="cyber-card sticky top-4 h-[calc(100vh-2rem)] w-full p-4 lg:w-64">
      <div className="mb-8 flex items-center gap-3">
        <div className="grid h-11 w-11 place-items-center rounded-md border border-cyber-cyan/40 bg-cyber-cyan/10 text-cyber-cyan shadow-glow">
          <Bot size={22} />
        </div>
        <div>
          <p className="font-orbitron text-sm text-cyber-muted">POLYMARKET</p>
          <h1 className="font-orbitron text-lg font-bold text-cyber-text">Command Deck</h1>
        </div>
      </div>

      <nav className="grid gap-2">
        {items.map((item) => {
          const Icon = item.icon
          const isActive = active === item.key

          return (
            <button
              key={item.key}
              type="button"
              onClick={() => onNavigate(item.key)}
              className={`flex items-center gap-3 rounded-md border px-3 py-3 text-left text-sm transition ${
                isActive
                  ? 'border-cyber-cyan/50 bg-cyber-cyan/12 text-cyber-cyan shadow-glow'
                  : 'border-transparent text-cyber-muted hover:border-cyber-cyan/25 hover:bg-cyber-card hover:text-cyber-text'
              }`}
              title={item.label}
            >
              <Icon size={18} />
              <span>{item.label}</span>
            </button>
          )
        })}
      </nav>
    </aside>
  )
}
