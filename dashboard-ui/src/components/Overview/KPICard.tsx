import type { LucideIcon } from 'lucide-react'

type KPICardProps = {
  label: string
  value: string
  tone: 'cyan' | 'green' | 'red' | 'magenta' | 'yellow'
  icon: LucideIcon
}

const toneClass = {
  cyan: 'text-cyber-cyan border-cyber-cyan/30 bg-cyber-cyan/10',
  green: 'text-cyber-green border-cyber-green/30 bg-cyber-green/10',
  red: 'text-cyber-red border-cyber-red/30 bg-cyber-red/10',
  magenta: 'text-cyber-magenta border-cyber-magenta/30 bg-cyber-magenta/10',
  yellow: 'text-cyber-yellow border-cyber-yellow/30 bg-cyber-yellow/10'
}

export function KPICard({ label, value, tone, icon: Icon }: KPICardProps) {
  return (
    <div className="cyber-card p-4">
      <div className="mb-5 flex items-center justify-between">
        <span className="text-xs uppercase text-cyber-muted">{label}</span>
        <span className={`grid h-9 w-9 place-items-center rounded-md border ${toneClass[tone]}`}>
          <Icon size={18} />
        </span>
      </div>
      <p className={`font-orbitron text-2xl font-bold ${toneClass[tone].split(' ')[0]}`}>{value}</p>
    </div>
  )
}
