import axios from 'axios'
import type { BetHistory, GlobalConfig, Health, OpenBet, Overview, Signal, Strategy, StrategyPatch } from '../types'

// Default to same-origin so the nginx reverse proxy (prod) and the Vite dev
// proxy both serve /api without CORS. Override with VITE_API_URL to point at a
// separately-hosted API.
export const API_BASE =
  import.meta.env.VITE_API_URL || (typeof window !== 'undefined' ? window.location.origin : 'http://localhost:8001')

export const api = axios.create({
  baseURL: API_BASE,
  headers: { 'Content-Type': 'application/json' }
})

const unwrap = <T>(request: Promise<{ data: T }>) => request.then((response) => response.data)

export const getHealth = () => unwrap<Health>(api.get('/api/health'))

export const getOverview = () => unwrap<Overview>(api.get('/api/overview'))

export const getOpenBets = () => unwrap<OpenBet[]>(api.get('/api/bets/open'))

export const getBetHistory = (params: { limit?: number; offset?: number; from?: string; to?: string } = {}) =>
  unwrap<BetHistory[]>(api.get('/api/bets/history', { params }))

export const getRecentSignals = (limit = 20) => unwrap<Signal[]>(api.get('/api/signals/recent', { params: { limit } }))

export const getStrategies = () => unwrap<Strategy[]>(api.get('/api/strategies'))

export const updateStrategy = (name: string, data: StrategyPatch) => unwrap<Strategy>(api.put(`/api/strategies/${name}`, data))

export const getGlobalConfig = () => unwrap<GlobalConfig>(api.get('/api/config/global'))

export const updateGlobalConfig = (data: GlobalConfig) => unwrap<GlobalConfig>(api.put('/api/config/global', data))

export const wsLogsUrl = () => {
  const url = new URL(API_BASE)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  url.pathname = '/api/logs/stream'
  return url.toString()
}
