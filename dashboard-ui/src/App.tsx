import { Activity, BarChart3, BrainCircuit, CircleDollarSign, Gauge, RadioTower, TrendingUp, WalletCards } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useState } from 'react'
import { HistoryTable } from './components/Bets/HistoryTable'
import { OpenBetsTable } from './components/Bets/OpenBetsTable'
import { GlobalConfig } from './components/Config/GlobalConfig'
import { Header } from './components/Layout/Header'
import { Sidebar } from './components/Layout/Sidebar'
import { LogStream } from './components/Logs/LogStream'
import { KPICard } from './components/Overview/KPICard'
import { LatencyCard } from './components/Overview/LatencyCard'
import { PnLChart } from './components/Overview/PnLChart'
import { SignalTable } from './components/Signals/SignalTable'
import { StrategyCard } from './components/Strategies/StrategyCard'
import { StrategyEditor } from './components/Strategies/StrategyEditor'
import { useApi } from './hooks/useApi'
import {
  getBetHistory,
  getGlobalConfig,
  getHealth,
  getLatencyMetrics,
  getOpenBets,
  getOverview,
  getRecentSignals,
  getStrategies,
  updateGlobalConfig,
  updateStrategy
} from './services/api'
import type {
  BetHistory,
  GlobalConfig as GlobalConfigType,
  LatencyMetrics,
  NavKey,
  Overview,
  Strategy,
  StrategyPatch
} from './types'
import { formatCurrency, formatPercent } from './utils/format'

export default function App() {
  const [active, setActive] = useState<NavKey>('overview')
  const [editingStrategy, setEditingStrategy] = useState<Strategy | null>(null)
  const [savingStrategy, setSavingStrategy] = useState(false)
  const [savingConfig, setSavingConfig] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)

  const health = useApi(getHealth, 5000)
  const overview = useApi(getOverview, 10000)
  const openBets = useApi(getOpenBets, 10000)
  const betHistory = useApi(() => getBetHistory({ limit: 50, offset: 0 }), 15000)
  const signals = useApi(() => getRecentSignals(20), 10000)
  const strategies = useApi(getStrategies, 15000)
  const globalConfig = useApi(getGlobalConfig, 15000)
  const latency = useApi(getLatencyMetrics, 15000)

  const saveStrategy = async (name: string, patch: StrategyPatch) => {
    setSavingStrategy(true)
    setSaveError(null)
    try {
      const updated = await updateStrategy(name, patch)
      strategies.setData((strategies.data ?? []).map((strategy) => (strategy.name === updated.name ? updated : strategy)))
      setEditingStrategy(null)
    } catch (error) {
      setSaveError(`Failed to save strategy "${name}": ${describeSaveError(error)}`)
    } finally {
      setSavingStrategy(false)
    }
  }

  const saveGlobalConfig = async (patch: GlobalConfigType) => {
    setSavingConfig(true)
    setSaveError(null)
    try {
      const updated = await updateGlobalConfig(patch)
      globalConfig.setData(updated)
    } catch (error) {
      setSaveError(`Failed to save global config: ${describeSaveError(error)}`)
    } finally {
      setSavingConfig(false)
    }
  }

  const toggleStrategyActive = async (strategy: Strategy) => {
    const currentConfig = globalConfig.data ?? {}
    const allStrategyNames = (strategies.data ?? []).map((item) => item.name.toLowerCase())
    const currentActive = activeStrategies(currentConfig, allStrategyNames)
    const key = strategy.name.toLowerCase()
    const nextActive = currentActive.includes(key) ? currentActive.filter((name) => name !== key) : [...currentActive, key]
    await saveGlobalConfig({ ...currentConfig, active_strategies: nextActive })
  }

  return (
    <div className="min-h-screen bg-scan-grid bg-[length:42px_42px] p-3 md:p-4">
      <div className="mx-auto flex max-w-[1800px] flex-col gap-4 lg:flex-row">
        <Sidebar active={active} onNavigate={setActive} />

        <main className="grid min-w-0 flex-1 gap-4">
          <Header health={health.data} />
          <StatusLine
            errors={[
              health.error,
              overview.error,
              openBets.error,
              betHistory.error,
              signals.error,
              strategies.error,
              globalConfig.error,
              latency.error
            ]}
          />
          {saveError ? <SaveErrorBanner message={saveError} onDismiss={() => setSaveError(null)} /> : null}

          {active === 'overview' ? (
            <OverviewPanel
              overview={overview.data}
              history={betHistory.data ?? []}
              loading={overview.loading}
              latency={latency.data}
              latencyLoading={latency.loading}
            />
          ) : null}

          {active === 'bets' ? (
            <div className="grid gap-4">
              <OpenBetsTable bets={openBets.data ?? []} />
              <HistoryTable bets={betHistory.data ?? []} />
            </div>
          ) : null}

          {active === 'strategies' ? (
            <div className="grid gap-4">
              <div className="cyber-card p-4">
                <h3 className="font-orbitron text-lg text-cyber-text">Strategy Selector</h3>
                <p className="mt-1 text-sm text-cyber-muted">Enabled strategies are used by the Rust bot after next runtime-config polling cycle.</p>
              </div>
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                {(strategies.data ?? []).map((strategy) => {
                  const enabled = activeStrategies(
                    globalConfig.data ?? {},
                    (strategies.data ?? []).map((item) => item.name.toLowerCase())
                  ).includes(strategy.name.toLowerCase())

                  return (
                    <StrategyCard
                      key={strategy.name}
                      strategy={strategy}
                      active={enabled}
                      onEdit={setEditingStrategy}
                      onToggleActive={toggleStrategyActive}
                    />
                  )
                })}
              </div>
              {strategies.data?.length === 0 ? <EmptyState label="No strategies loaded" /> : null}
            </div>
          ) : null}

          {active === 'signals' ? <SignalTable signals={signals.data ?? []} /> : null}

          {active === 'config' ? <GlobalConfig config={globalConfig.data} saving={savingConfig} onSave={saveGlobalConfig} /> : null}

          {active === 'logs' ? <LogStream /> : null}
        </main>
      </div>

      <StrategyEditor strategy={editingStrategy} saving={savingStrategy} onClose={() => setEditingStrategy(null)} onSave={saveStrategy} />
    </div>
  )
}

function OverviewPanel({
  overview,
  history,
  loading,
  latency,
  latencyLoading
}: {
  overview: Overview | null
  history: BetHistory[]
  loading: boolean
  latency: LatencyMetrics | null
  latencyLoading: boolean
}) {
  if (!overview && loading) {
    return <EmptyState label="Loading telemetry" />
  }

  const data = overview ?? {
    total_bankroll: 0,
    pnl_today: 0,
    pnl_week: 0,
    open_bets: 0,
    total_bets: 0,
    win_rate: 0,
    profit_factor: 0,
    signals_today: 0,
    last_scan: null
  }

  return (
    <div className="grid gap-4">
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-6">
        <KPICard label="Bankroll" value={formatCurrency(data.total_bankroll)} tone="cyan" icon={CircleDollarSign} />
        <KPICard label="P&L Today" value={formatCurrency(data.pnl_today)} tone={data.pnl_today >= 0 ? 'green' : 'red'} icon={TrendingUp} />
        <KPICard label="P&L Week" value={formatCurrency(data.pnl_week)} tone={data.pnl_week >= 0 ? 'green' : 'red'} icon={BarChart3} />
        <KPICard label="Open Bets" value={String(data.open_bets)} tone="magenta" icon={WalletCards} />
        <KPICard label="Win Rate" value={formatPercent(data.win_rate)} tone="yellow" icon={Gauge} />
        <KPICard label="Profit Factor" value={data.profit_factor.toFixed(2)} tone="green" icon={BrainCircuit} />
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.4fr_0.6fr]">
        <PnLChart history={history} />
        <div className="cyber-card grid content-start gap-4 p-4">
          <h3 className="font-orbitron text-lg text-cyber-text">Scanner Pulse</h3>
          <StatLine icon={RadioTower} label="Signals today" value={String(data.signals_today)} />
          <StatLine icon={Activity} label="Total bets" value={String(data.total_bets)} />
          <StatLine icon={BarChart3} label="Last scan" value={data.last_scan ? new Date(data.last_scan).toLocaleString() : '-'} />
        </div>
      </div>

      <LatencyCard metrics={latency} loading={latencyLoading} />
    </div>
  )
}

function StatLine({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-md border border-cyber-border bg-black/20 px-4 py-3">
      <span className="flex items-center gap-2 text-sm text-cyber-muted">
        <Icon size={16} />
        {label}
      </span>
      <span className="text-right font-orbitron text-cyber-cyan">{value}</span>
    </div>
  )
}

function StatusLine({ errors }: { errors: Array<string | null> }) {
  const activeErrors = errors.filter(Boolean)

  if (activeErrors.length === 0) {
    return null
  }

  return (
    <div className="rounded-md border border-cyber-yellow/30 bg-cyber-yellow/10 px-4 py-3 text-sm text-cyber-yellow">
      {activeErrors[0]}
    </div>
  )
}

function EmptyState({ label }: { label: string }) {
  return <div className="cyber-card p-8 text-center text-sm text-cyber-muted">{label}</div>
}

function SaveErrorBanner({ message, onDismiss }: { message: string; onDismiss: () => void }) {
  return (
    <div className="flex items-start justify-between gap-4 rounded-md border border-cyber-red/40 bg-cyber-red/10 px-4 py-3 text-sm text-cyber-red">
      <span>{message}</span>
      <button type="button" onClick={onDismiss} className="shrink-0 text-cyber-red/70 hover:text-cyber-red">
        Dismiss
      </button>
    </div>
  )
}

function describeSaveError(error: unknown): string {
  if (isAxiosLikeError(error)) {
    const detail = error.response?.data?.detail
    if (typeof detail === 'string') {
      return detail
    }
    if (Array.isArray(detail) && detail.length > 0) {
      // Pydantic 422 returns a list of {loc, msg, type}.
      return detail.map((item) => (item?.loc ? `${item.loc.join('.')}: ${item.msg}` : item.msg)).join('; ')
    }
    if (error.response?.status) {
      return `HTTP ${error.response.status}`
    }
  }
  if (error instanceof Error) {
    return error.message
  }
  return 'unknown error'
}

type AxiosLikeError = {
  response?: {
    status?: number
    data?: { detail?: unknown }
  }
}

function isAxiosLikeError(error: unknown): error is AxiosLikeError {
  return typeof error === 'object' && error !== null && 'response' in error
}

function activeStrategies(config: GlobalConfigType, fallback: string[]) {
  const configured = config.active_strategies
  if (Array.isArray(configured) && configured.length > 0) {
    return configured.map((name) => String(name).toLowerCase())
  }
  return fallback
}
