import { Area, AreaChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { BetHistory } from '../../types'

type PnLChartProps = {
  history: BetHistory[]
}

export function PnLChart({ history }: PnLChartProps) {
  const data = buildPnlSeries(history)

  return (
    <div className="cyber-card min-h-[320px] p-4">
      <div className="mb-4">
        <h3 className="font-orbitron text-lg text-cyber-text">7D P&amp;L Vector</h3>
        <p className="text-sm text-cyber-muted">Resolved bets grouped by day</p>
      </div>

      <div className="h-64">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 10, right: 18, left: 0, bottom: 0 }}>
            <defs>
              <linearGradient id="pnlGradient" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="#00f3ff" stopOpacity={0.55} />
                <stop offset="100%" stopColor="#ff00e6" stopOpacity={0.04} />
              </linearGradient>
            </defs>
            <CartesianGrid stroke="rgba(136,136,170,0.16)" vertical={false} />
            <XAxis dataKey="date" stroke="#8888aa" tickLine={false} axisLine={false} />
            <YAxis stroke="#8888aa" tickLine={false} axisLine={false} width={48} />
            <Tooltip
              contentStyle={{
                background: '#12121a',
                border: '1px solid rgba(0,243,255,0.3)',
                borderRadius: 8,
                color: '#e0e0e0'
              }}
            />
            <Area type="monotone" dataKey="pnl" stroke="#00f3ff" strokeWidth={2} fill="url(#pnlGradient)" />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

function buildPnlSeries(history: BetHistory[]) {
  const days = [...Array(7)].map((_, index) => {
    const date = new Date()
    date.setDate(date.getDate() - (6 - index))
    const key = date.toISOString().slice(0, 10)
    return { key, date: `${date.getMonth() + 1}/${date.getDate()}`, pnl: 0 }
  })

  const byKey = new Map(days.map((day) => [day.key, day]))
  for (const bet of history) {
    const key = bet.resolved_at?.slice(0, 10)
    if (key && byKey.has(key)) {
      const day = byKey.get(key)!
      day.pnl += bet.pnl ?? 0
    }
  }

  return days
}
