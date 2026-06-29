import { Flame, ShieldAlert, Save, SlidersHorizontal, TrendingUp } from 'lucide-react'
import type { FormEvent } from 'react'
import { useEffect, useState } from 'react'
import type { GlobalConfig as GlobalConfigType } from '../../types'

type GlobalConfigProps = {
  config: GlobalConfigType | null
  saving: boolean
  onSave: (patch: GlobalConfigType) => Promise<void>
}

type NumericField = {
  key: string
  label: string
  min: number
  max?: number
  step: number
  suffix?: string
}

const operationalFields: NumericField[] = [
  { key: 'scan_interval_mins', label: 'Housekeeping interval', min: 1, step: 1, suffix: 'min' },
  { key: 'bet_scan_interval_mins', label: 'Bet scan interval', min: 1, step: 1, suffix: 'min' },
  { key: 'heartbeat_interval_mins', label: 'Heartbeat interval', min: 0, step: 1, suffix: 'min' },
  { key: 'config_poll_interval_secs', label: 'Runtime config polling', min: 1, step: 5, suffix: 'sec' },
  { key: 'min_volume', label: 'Minimum market volume', min: 0, step: 100, suffix: 'USD' },
  { key: 'max_markets_fetch', label: 'Max markets fetched', min: 1, step: 50 }
]

const riskFields: NumericField[] = [
  { key: 'strategy_bankroll', label: 'Stake bankroll per strategy', min: 0, step: 10, suffix: 'EUR' },
  { key: 'kelly_fraction', label: 'Global Kelly fraction', min: 0, max: 1, step: 0.01 },
  { key: 'min_kelly_size', label: 'Minimum Kelly gate', min: 0, max: 1, step: 0.005 },
  { key: 'min_bet_price', label: 'Minimum entry price', min: 0, max: 1, step: 0.01 },
  { key: 'stop_loss_pct', label: 'Stop loss', min: 0, step: 0.05 },
  { key: 'take_profit_pct', label: 'Take profit', min: 0, step: 0.05 },
  { key: 'exit_days_before_expiry', label: 'Exit days before expiry', min: 0, step: 1, suffix: 'days' },
  { key: 'max_ws_bets_per_day', label: 'Max WS bets per day', min: 0, step: 1 },
  { key: 'ws_bet_cooldown_secs', label: 'WS bet cooldown', min: 0, step: 30, suffix: 'sec' },
  { key: 'slippage_pct', label: 'Slippage assumption', min: 0, max: 1, step: 0.005 }
]

const defaultFormConfig: GlobalConfigType = {
  config_poll_interval_secs: 60,
  take_profit_pct: 0
}

const riskProfiles: Array<{ key: string; label: string; patch: GlobalConfigType }> = [
  {
    key: 'conservative',
    label: 'Conservative',
    patch: {
      risk_profile: 'conservative',
      kelly_fraction: 0.15,
      min_kelly_size: 0.03,
      stop_loss_pct: 0.25,
      take_profit_pct: 0.75,
      max_ws_bets_per_day: 2,
      ws_bet_cooldown_secs: 900,
      slippage_pct: 0.005
    }
  },
  {
    key: 'balanced',
    label: 'Balanced',
    patch: {
      risk_profile: 'balanced',
      kelly_fraction: 0.25,
      min_kelly_size: 0.02,
      stop_loss_pct: 0.5,
      take_profit_pct: 1,
      max_ws_bets_per_day: 3,
      ws_bet_cooldown_secs: 600,
      slippage_pct: 0.01
    }
  },
  {
    key: 'aggressive',
    label: 'Aggressive',
    patch: {
      risk_profile: 'aggressive',
      kelly_fraction: 0.5,
      min_kelly_size: 0.01,
      stop_loss_pct: 0.75,
      take_profit_pct: 1.5,
      max_ws_bets_per_day: 6,
      ws_bet_cooldown_secs: 300,
      slippage_pct: 0.015
    }
  }
]

export function GlobalConfig({ config, saving, onSave }: GlobalConfigProps) {
  const [form, setForm] = useState<GlobalConfigType>({})

  useEffect(() => {
    setForm({
      ...defaultFormConfig,
      ...(config ?? {})
    })
  }, [config])

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    await onSave(form)
  }

  return (
    <form onSubmit={submit} className="grid gap-4">
      <section className="cyber-card p-4">
        <SectionHeader icon={SlidersHorizontal} title="Global Runtime Config" subtitle="Saved to FastAPI runtime_config" />

        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {operationalFields.map((field) => (
            <NumberField key={field.key} field={field} value={Number(form[field.key] ?? 0)} onChange={(value) => setField(field.key, value)} />
          ))}

          <label className="grid gap-2 text-sm text-cyber-muted md:col-span-2">
            model_sidecar_url
            <input
              type="url"
              value={String(form.model_sidecar_url ?? '')}
              onChange={(event) => setForm((current) => ({ ...current, model_sidecar_url: event.target.value }))}
              className="cyber-input px-3 py-3"
            />
          </label>

          <label className="flex items-center justify-between gap-4 rounded-md border border-cyber-border bg-black/20 px-4 py-3 text-sm text-cyber-muted">
            news_enabled
            <input
              type="checkbox"
              checked={Boolean(form.news_enabled)}
              onChange={(event) => setForm((current) => ({ ...current, news_enabled: event.target.checked }))}
              className="h-5 w-5 accent-cyber-cyan"
            />
          </label>
        </div>
      </section>

      <section className="cyber-card p-4">
        <SectionHeader icon={ShieldAlert} title="Financial Risk Controls" subtitle="Position sizing, early exit, and exposure guardrails" />

        <div className="mb-4 grid gap-2 md:grid-cols-3">
          {riskProfiles.map((profile) => {
            const selected = form.risk_profile === profile.key

            return (
              <button
                key={profile.key}
                type="button"
                onClick={() => applyRiskProfile(profile.patch)}
                className={`flex items-center justify-between rounded-md border px-4 py-3 text-left text-sm transition ${
                  selected
                    ? 'border-cyber-magenta/50 bg-cyber-magenta/12 text-cyber-magenta shadow-glow'
                    : 'border-cyber-border bg-black/20 text-cyber-muted hover:border-cyber-cyan/30 hover:text-cyber-text'
                }`}
              >
                <span>{profile.label}</span>
                <Flame size={16} />
              </button>
            )
          })}
        </div>

        <div className="mb-4 grid gap-3 md:grid-cols-3">
          <RiskReadout label="Stop loss" value={riskDisplay('stop_loss_pct')} tone="red" />
          <RiskReadout label="Take profit" value={riskDisplay('take_profit_pct')} tone="green" />
          <RiskReadout label="Stake bankroll" value={`${Number(form.strategy_bankroll ?? 0).toFixed(2)} EUR`} tone="cyan" />
        </div>

        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {riskFields.map((field) => (
            <NumberField key={field.key} field={field} value={Number(form[field.key] ?? 0)} onChange={(value) => setRiskField(field.key, value)} />
          ))}
        </div>

        <div className="mt-4 rounded-md border border-cyber-yellow/30 bg-cyber-yellow/10 p-3 text-sm text-cyber-yellow">
          `take_profit_pct` and global `kelly_fraction` are dashboard compatibility fields; live Kelly sizing is controlled per strategy.
        </div>
      </section>

      <div className="mt-6 flex justify-end">
        <button
          type="submit"
          disabled={saving}
          className="inline-flex items-center gap-2 rounded-md border border-cyber-green/40 bg-cyber-green/10 px-4 py-3 text-sm text-cyber-green hover:bg-cyber-green/20 disabled:cursor-not-allowed disabled:opacity-60"
        >
          <Save size={16} />
          {saving ? 'Saving' : 'Save'}
        </button>
      </div>
    </form>
  )

  function setField(key: string, value: number) {
    setForm((current) => ({ ...current, [key]: value }))
  }

  function setRiskField(key: string, value: number) {
    setForm((current) => ({ ...current, [key]: value, risk_profile: 'custom' }))
  }

  function applyRiskProfile(patch: GlobalConfigType) {
    setForm((current) => ({ ...current, ...patch }))
  }

  function riskDisplay(key: string) {
    const value = Number(form[key] ?? 0)
    if (key === 'stop_loss_pct' && value >= 999) {
      return 'Disabled'
    }
    if (value === 0) {
      return 'Off'
    }
    return `${(value * 100).toFixed(0)}%`
  }
}

function SectionHeader({ icon: Icon, title, subtitle }: { icon: typeof Save; title: string; subtitle: string }) {
  return (
    <div className="mb-5 flex items-start gap-3">
      <div className="grid h-10 w-10 place-items-center rounded-md border border-cyber-cyan/35 bg-cyber-cyan/10 text-cyber-cyan">
        <Icon size={18} />
      </div>
      <div>
        <h3 className="font-orbitron text-lg text-cyber-text">{title}</h3>
        <p className="text-sm text-cyber-muted">{subtitle}</p>
      </div>
    </div>
  )
}

function NumberField({
  field,
  value,
  onChange
}: {
  field: NumericField
  value: number
  onChange: (value: number) => void
}) {
  return (
    <label className="grid gap-2 text-sm text-cyber-muted">
      <span className="flex items-center justify-between gap-3">
        {field.label}
        {field.suffix ? <span className="text-xs text-cyber-cyan">{field.suffix}</span> : null}
      </span>
      <input
        type="number"
        min={field.min}
        max={field.max}
        step={field.step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="cyber-input px-3 py-3"
      />
    </label>
  )
}

function RiskReadout({ label, value, tone }: { label: string; value: string; tone: 'red' | 'green' | 'cyan' }) {
  const toneClass = {
    red: 'border-cyber-red/30 bg-cyber-red/10 text-cyber-red',
    green: 'border-cyber-green/30 bg-cyber-green/10 text-cyber-green',
    cyan: 'border-cyber-cyan/30 bg-cyber-cyan/10 text-cyber-cyan'
  }[tone]

  return (
    <div className={`rounded-md border px-4 py-3 ${toneClass}`}>
      <div className="mb-1 flex items-center gap-2 text-xs uppercase">
        <TrendingUp size={14} />
        {label}
      </div>
      <p className="font-orbitron text-xl">{value}</p>
    </div>
  )
}
