import { Circle, Filter } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { wsLogsUrl } from '../../services/api'
import { useWebSocket } from '../../hooks/useWebSocket'
import { formatDate } from '../../utils/format'

const levels = ['ALL', 'INFO', 'WARN', 'ERROR']

export function LogStream() {
  const { messages, connected } = useWebSocket(wsLogsUrl())
  const [level, setLevel] = useState('ALL')
  const endRef = useRef<HTMLDivElement>(null)

  const visible = level === 'ALL' ? messages : messages.filter((message) => message.level.toUpperCase() === level)

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [visible.length])

  return (
    <div className="cyber-card p-4">
      <div className="mb-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 className="font-orbitron text-lg text-cyber-text">Live Logs</h3>
          <p className="flex items-center gap-2 text-sm text-cyber-muted">
            <Circle size={10} className={connected ? 'fill-cyber-green text-cyber-green' : 'fill-cyber-red text-cyber-red'} />
            {connected ? 'Connected' : 'Disconnected'}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Filter size={16} className="text-cyber-muted" />
          {levels.map((item) => (
            <button
              key={item}
              type="button"
              onClick={() => setLevel(item)}
              className={`rounded-md border px-3 py-2 text-xs ${
                level === item
                  ? 'border-cyber-cyan/45 bg-cyber-cyan/10 text-cyber-cyan'
                  : 'border-cyber-border text-cyber-muted hover:text-cyber-text'
              }`}
            >
              {item}
            </button>
          ))}
        </div>
      </div>

      <div className="cyber-scroll h-[520px] overflow-auto rounded-md border border-cyber-border bg-black/50 p-4 font-mono text-sm">
        {visible.map((message, index) => (
          <div key={`${message.timestamp}-${index}`} className="mb-3 grid gap-1 border-b border-cyber-border/40 pb-3">
            <div className="flex flex-wrap items-center gap-3 text-xs">
              <span className="text-cyber-muted">{formatDate(message.timestamp)}</span>
              <span className={levelColor(message.level)}>{message.level.toUpperCase()}</span>
              <span className="text-cyber-cyan">{message.target}</span>
            </div>
            <p className="break-words text-cyber-text">{message.message}</p>
            {Object.keys(message.fields).length > 0 ? (
              <pre className="cyber-scroll overflow-x-auto text-xs text-cyber-muted">{JSON.stringify(message.fields, null, 2)}</pre>
            ) : null}
          </div>
        ))}
        {visible.length === 0 ? <p className="text-cyber-muted">No log entries</p> : null}
        <div ref={endRef} />
      </div>
    </div>
  )
}

function levelColor(level: string) {
  const normalized = level.toUpperCase()
  if (normalized === 'ERROR') return 'text-cyber-red'
  if (normalized === 'WARN' || normalized === 'WARNING') return 'text-cyber-yellow'
  return 'text-cyber-green'
}
