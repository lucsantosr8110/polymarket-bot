import { Clock, Server } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { Health } from '../../types'
import { API_BASE } from '../../services/api'
import { PulseBadge, StatusBadge } from './StatusBadge'

type HeaderProps = {
  health: Health | null
}

export function Header({ health }: HeaderProps) {
  const [now, setNow] = useState(() => new Date())

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000)
    return () => window.clearInterval(timer)
  }, [])

  return (
    <header className="cyber-card flex flex-col gap-4 p-4 lg:flex-row lg:items-center lg:justify-between">
      <div>
        <p className="mb-1 flex items-center gap-2 text-xs uppercase text-cyber-muted">
          <Server size={14} />
          {API_BASE}
        </p>
        <h2 className="font-orbitron text-xl font-bold text-cyber-text">Trading telemetry</h2>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <StatusBadge label="DB" active={health?.db_connected ?? false} />
        <StatusBadge label="Model" active={health?.model_loaded ?? false} />
        <StatusBadge label="Bot" active={health?.bot_running ?? false} />
        <PulseBadge label={`${health?.uptime_seconds ?? 0}s`} />
        <span className="inline-flex items-center gap-2 rounded-md border border-cyber-border bg-black/20 px-3 py-2 text-xs text-cyber-muted">
          <Clock size={14} />
          {now.toLocaleTimeString()}
        </span>
      </div>
    </header>
  )
}
