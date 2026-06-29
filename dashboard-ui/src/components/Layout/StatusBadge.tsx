import { Activity, CircleAlert, CircleCheck } from 'lucide-react'

type StatusBadgeProps = {
  label: string
  active: boolean
}

export function StatusBadge({ label, active }: StatusBadgeProps) {
  const Icon = active ? CircleCheck : CircleAlert

  return (
    <span
      className={`inline-flex items-center gap-2 rounded-md border px-3 py-2 text-xs ${
        active
          ? 'border-cyber-green/40 bg-cyber-green/10 text-cyber-green'
          : 'border-cyber-red/40 bg-cyber-red/10 text-cyber-red'
      }`}
    >
      <Icon size={14} />
      {label}
    </span>
  )
}

export function PulseBadge({ label }: { label: string }) {
  return (
    <span className="inline-flex items-center gap-2 rounded-md border border-cyber-cyan/30 bg-cyber-cyan/10 px-3 py-2 text-xs text-cyber-cyan">
      <Activity size={14} className="animate-pulse" />
      {label}
    </span>
  )
}
