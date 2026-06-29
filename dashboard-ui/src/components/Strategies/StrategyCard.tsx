import { Gauge, Pencil, Power, Target } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import type { Strategy } from '../../types'
import { formatCurrency, formatPercent } from '../../utils/format'

type StrategyCardProps = {
  strategy: Strategy
  active: boolean
  onEdit: (strategy: Strategy) => void
  onToggleActive: (strategy: Strategy) => void
}

export function StrategyCard({ strategy, active, onEdit, onToggleActive }: StrategyCardProps) {
  return (
    <article className={`cyber-card p-4 ${active ? '' : 'opacity-60'}`}>
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <p className="text-xs uppercase text-cyber-muted">Strategy</p>
          <h3 className="font-orbitron text-xl text-cyber-cyan">{strategy.name}</h3>
          <p className={active ? 'text-xs text-cyber-green' : 'text-xs text-cyber-red'}>{active ? 'Active' : 'Disabled'}</p>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => onToggleActive(strategy)}
            className={`grid h-10 w-10 place-items-center rounded-md border ${
              active
                ? 'border-cyber-green/40 bg-cyber-green/10 text-cyber-green hover:bg-cyber-green/20'
                : 'border-cyber-red/40 bg-cyber-red/10 text-cyber-red hover:bg-cyber-red/20'
            }`}
            title={active ? `Disable ${strategy.name}` : `Enable ${strategy.name}`}
          >
            <Power size={17} />
          </button>
          <button
            type="button"
            onClick={() => onEdit(strategy)}
            className="grid h-10 w-10 place-items-center rounded-md border border-cyber-cyan/35 bg-cyber-cyan/10 text-cyber-cyan hover:bg-cyber-cyan/20"
            title={`Edit ${strategy.name}`}
          >
            <Pencil size={17} />
          </button>
        </div>
      </div>

      <div className="grid gap-3 text-sm">
        <Metric icon={Target} label="Min edge" value={formatPercent(strategy.min_edge)} />
        <Metric icon={Gauge} label="Confidence" value={formatPercent(strategy.min_confidence)} />
        <Metric icon={Gauge} label="Kelly" value={formatPercent(strategy.kelly_fraction)} />
        <Metric icon={Target} label="Daily cap" value={String(strategy.max_signals_per_day)} />
        <Metric icon={Target} label="Min bet" value={formatCurrency(strategy.min_bet)} />
      </div>
    </article>
  )
}

function Metric({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-md border border-cyber-border/70 bg-black/20 px-3 py-2">
      <span className="flex items-center gap-2 text-cyber-muted">
        <Icon size={14} />
        {label}
      </span>
      <span className="font-orbitron text-cyber-text">{value}</span>
    </div>
  )
}
