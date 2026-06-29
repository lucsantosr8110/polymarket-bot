import { useEffect, useState } from 'react'
import type { LogEntry } from '../types'

export const useWebSocket = (url: string) => {
  const [messages, setMessages] = useState<LogEntry[]>([])
  const [connected, setConnected] = useState(false)

  useEffect(() => {
    const ws = new WebSocket(url)

    ws.onopen = () => setConnected(true)
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as LogEntry
        setMessages((prev) => [...prev.slice(-299), data])
      } catch (error) {
        console.warn('Failed to parse log message', error)
      }
    }
    ws.onclose = () => setConnected(false)
    ws.onerror = () => setConnected(false)

    return () => ws.close()
  }, [url])

  return { messages, connected }
}
