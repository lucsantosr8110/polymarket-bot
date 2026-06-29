export type Health = {
  status: 'ok'
  db_connected: boolean
  model_loaded: boolean
  bot_running: boolean
  uptime_seconds: number
}

export type Overview = {
  total_bankroll: number
  pnl_today: number
  pnl_week: number
  open_bets: number
  total_bets: number
  win_rate: number
  profit_factor: number
  signals_today: number
  last_scan: string | null
}

export type OpenBet = {
  id: number
  market_id: string
  question: string
  side: string
  entry_price: number
  current_price: number
  shares: number
  cost: number
  pnl_unrealized: number
  placed_at: string
  category?: string | null
  fee_paid?: number | null
  fee_rate?: number | null
}

export type BetHistory = {
  id: number
  market_id: string
  question: string
  side: string
  entry_price: number
  shares: number
  cost: number
  pnl: number | null
  won: boolean | null
  placed_at: string
  resolved_at: string | null
  category?: string | null
  fee_paid?: number | null
  fee_rate?: number | null
}

export type Signal = {
  status: 'accepted' | 'rejected'
  market_id: string
  question: string
  reason: string | null
  side: string | null
  entry_price: number | null
  current_price: number | null
  estimated_prob: number | null
  edge: number | null
  confidence: number | null
  created_at: string
}

export type Strategy = {
  name: string
  min_edge: number
  min_confidence: number
  kelly_fraction: number
  max_signals_per_day: number
  min_bet: number
}

export type StrategyPatch = Partial<Omit<Strategy, 'name'>>

export type GlobalConfig = {
  scan_interval_mins?: number
  bet_scan_interval_mins?: number
  heartbeat_interval_mins?: number
  config_poll_interval_secs?: number
  active_strategies?: string[]
  risk_profile?: string
  model_sidecar_url?: string
  news_enabled?: boolean
  min_volume?: number
  max_markets_fetch?: number
  [key: string]: string | number | boolean | string[] | undefined
}

export type LogEntry = {
  timestamp: string
  level: string
  target: string
  message: string
  fields: Record<string, unknown>
}

export type NavKey = 'overview' | 'bets' | 'strategies' | 'signals' | 'config' | 'logs'
