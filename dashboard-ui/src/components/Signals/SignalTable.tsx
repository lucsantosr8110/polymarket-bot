import type { Signal } from '../../types'
import { formatDate, formatPercent, shortId } from '../../utils/format'

type SignalTableProps = {
  signals: Signal[]
}

export function SignalTable({ signals }: SignalTableProps) {
  return (
    <div className="cyber-card overflow-hidden p-4">
      <div className="mb-4">
        <h3 className="font-orbitron text-lg text-cyber-text">Signal Feed</h3>
        <p className="text-sm text-cyber-muted">{signals.length} latest decisions</p>
      </div>

      <div className="cyber-scroll overflow-x-auto">
        <table className="w-full min-w-[940px] text-left text-sm">
          <thead className="text-xs uppercase text-cyber-muted">
            <tr className="border-b border-cyber-border">
              <th className="py-3 pr-4">Timestamp</th>
              <th className="py-3 pr-4">Market</th>
              <th className="py-3 pr-4">Side</th>
              <th className="py-3 pr-4">Prob</th>
              <th className="py-3 pr-4">Edge</th>
              <th className="py-3 pr-4">Confidence</th>
              <th className="py-3 pr-4">Status</th>
              <th className="py-3">Reason</th>
            </tr>
          </thead>
          <tbody>
            {signals.map((signal, index) => (
              <tr key={`${signal.status}-${signal.market_id}-${index}`} className="border-b border-cyber-border/60 text-cyber-text">
                <td className="py-3 pr-4 text-cyber-muted">{formatDate(signal.created_at)}</td>
                <td className="max-w-[320px] py-3 pr-4">
                  <p className="truncate">{signal.question}</p>
                  <p className="text-xs text-cyber-muted">{shortId(signal.market_id)}</p>
                </td>
                <td className="py-3 pr-4">{signal.side ?? '-'}</td>
                <td className="py-3 pr-4">{signal.estimated_prob === null ? '-' : formatPercent(signal.estimated_prob)}</td>
                <td className="py-3 pr-4">{signal.edge === null ? '-' : formatPercent(signal.edge)}</td>
                <td className="py-3 pr-4">{signal.confidence === null ? '-' : formatPercent(signal.confidence)}</td>
                <td className={`py-3 pr-4 ${signal.status === 'accepted' ? 'text-cyber-green' : 'text-cyber-red'}`}>
                  {signal.status}
                </td>
                <td className="max-w-[260px] truncate py-3 text-cyber-muted">{signal.reason ?? '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {signals.length === 0 ? <p className="py-8 text-center text-sm text-cyber-muted">No signals</p> : null}
    </div>
  )
}
