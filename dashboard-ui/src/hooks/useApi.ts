import { useEffect, useRef, useState } from 'react'

export function useApi<T>(fetcher: () => Promise<T>, intervalMs?: number) {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const fetcherRef = useRef(fetcher)

  useEffect(() => {
    fetcherRef.current = fetcher
  })

  useEffect(() => {
    let active = true

    const load = async () => {
      setLoading(true)
      try {
        const result = await fetcherRef.current()
        if (active) {
          setData(result)
          setError(null)
        }
      } catch (err) {
        if (active) {
          setError(err instanceof Error ? err.message : 'Request failed')
        }
      } finally {
        if (active) {
          setLoading(false)
        }
      }
    }

    void load()
    const timer = intervalMs ? window.setInterval(load, intervalMs) : undefined

    return () => {
      active = false
      if (timer) {
        window.clearInterval(timer)
      }
    }
  }, [intervalMs])

  return { data, error, loading, setData }
}
