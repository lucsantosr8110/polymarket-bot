import type { BetHistory } from '../../types'
import { formatCurrency, formatDate, formatNumber, shortId } from '../../utils/format'

type HistoryTableProps = {
  bets: BetHistory[]
}

export function HistoryTable({ bets }: HistoryTableProps) {
  return (
    <div className="cyber-card overflow-hidden p-4">
      <div className="mb-4">
        <h3 className="font-orbitron text-lg text-cyber-text">Resolved History</h3>
        <p className="text-sm text-cyber-muted">{bets.length} latest settlements</p>
      </div>

      <div className="cyber-scroll overflow-x-auto">
        <table className="w-full min-w-[860px] text-left text-sm">
          <thead className="text-xs uppercase text-cyber-muted">
            <tr className="border-b border-cyber-border">
              <th className="py-3 pr-4">Market</th>
              <th className="py-3 pr-4">Side</th>
              <th className="py-3 pr-4">Entry</th>
              <th className="py-3 pr-4">Shares</th>
              <th className="py-3 pr-4">Cost</th>
              <th className="py-3 pr-4">P&amp;L</th>
              <th className="py-3 pr-4">Won</th>
              <th className="py-3">Resolved</th>
            </tr>
          </thead>
          <tbody>
            {bets.map((bet) => (
              <tr key={bet.id} className="border-b border-cyber-border/60 text-cyber-text">
                <td className="max-w-[340px] py-3 pr-4">
                  <p className="truncate">{bet.question}</p>
                  <p className="text-xs text-cyber-muted">{shortId(bet.market_id)}</p>
                </td>
                <td className="py-3 pr-4">{bet.side}</td>
                <td className="py-3 pr-4">{formatNumber(bet.entry_price)}</td>
                <td className="py-3 pr-4">{formatNumber(bet.shares)}</td>
                <td className="py-3 pr-4">{formatCurrency(bet.cost)}</td>
                <td className={`py-3 pr-4 ${(bet.pnl ?? 0) >= 0 ? 'text-cyber-green' : 'text-cyber-red'}`}>
                  {formatCurrency(bet.pnl ?? 0)}
                </td>
                <td className="py-3 pr-4">{bet.won === null ? 'Pending' : bet.won ? 'Yes' : 'No'}</td>
                <td className="py-3 text-cyber-muted">{bet.resolved_at ? formatDate(bet.resolved_at) : '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {bets.length === 0 ? <p className="py-8 text-center text-sm text-cyber-muted">No resolved bets</p> : null}
    </div>
  )
}
