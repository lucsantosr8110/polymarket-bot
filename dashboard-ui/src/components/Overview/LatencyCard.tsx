import { Activity, Clock } from 'lucide-react'
import type { LatencyMetrics } from '../../types'

type LatencyCardProps = {
  metrics: LatencyMetrics | null
  loading: boolean
}

type Row = { label: string; avg: number | null; p95: number | null }

function formatLatency(value: number | null): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return 'N/A'
  }
  return value < 1 ? `${(value * 1000).toFixed(0)}ms` : `${value.toFixed(2)}s`
}

export function LatencyCard({ metrics, loading }: LatencyCardProps) {
  const rows: Row[] = [
    { label: 'Bet scan', avg: metrics?.bet_scan_avg ?? null, p95: metrics?.bet_scan_p95 ?? null },
    { label: 'Fetch markets', avg: metrics?.fetch_markets_avg ?? null, p95: metrics?.fetch_markets_p95 ?? null },
    { label: 'Predict batch', avg: metrics?.predict_batch_avg ?? null, p95: metrics?.predict_batch_p95 ?? null },
    { label: 'Place bet', avg: metrics?.place_bet_avg ?? null, p95: metrics?.place_bet_p95 ?? null },
    { label: 'Housekeeping', avg: metrics?.housekeeping_avg ?? null, p95: metrics?.housekeeping_p95 ?? null },
    {
      label: 'Config poll',
      avg: metrics?.runtime_config_poll_avg ?? null,
      p95: metrics?.runtime_config_poll_p95 ?? null
    }
  ]

  return (
    <div className="cyber-card grid content-start gap-3 p-4">
      <h3 className="flex items-center gap-2 font-orbitron text-lg text-cyber-text">
        <Clock size={18} className="text-cyber-cyan" />
        Bot Latency
      </h3>

      {loading && !metrics ? (
        <p className="text-sm text-cyber-muted">Loading latency metrics...</p>
      ) : (
        <div className="grid gap-2">
          <div className="grid grid-cols-[1fr_auto_auto] gap-3 px-1 text-xs uppercase text-cyber-muted">
            <span>Operation</span>
            <span className="text-right">avg</span>
            <span className="text-right">p95</span>
          </div>
          {rows.map((row) => (
            <div
              key={row.label}
              className="grid grid-cols-[1fr_auto_auto] items-center gap-3 rounded-md border border-cyber-border bg-black/20 px-3 py-2"
            >
              <span className="flex items-center gap-2 text-sm text-cyber-muted">
                <Activity size={14} />
                {row.label}
              </span>
              <span className="text-right font-orbitron text-sm text-cyber-text">{formatLatency(row.avg)}</span>
              <span className="text-right font-orbitron text-sm text-cyber-cyan">{formatLatency(row.p95)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
