import { Clock, Flame, RadioTower, ShieldAlert, Save, SlidersHorizontal, TrendingUp } from 'lucide-react'
import type { FormEvent } from 'react'
import { useEffect, useState } from 'react'
import type { GlobalConfig as GlobalConfigType, RiskProfile } from '../../types'

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

// Every field below maps to a RuntimeGlobals member the Rust bot applies live
// via runtime-config polling. Nothing here requires a restart, and no field is
// shown that the bot would silently ignore.
const intervalFields: NumericField[] = [
  { key: 'scan_interval_mins', label: 'Housekeeping interval', min: 1, step: 1, suffix: 'min' },
  { key: 'bet_scan_interval_mins', label: 'Bet scan interval', min: 1, step: 1, suffix: 'min' },
  { key: 'heartbeat_interval_mins', label: 'Heartbeat interval', min: 0, step: 1, suffix: 'min' },
  { key: 'config_poll_interval_secs', label: 'Runtime config polling', min: 1, step: 5, suffix: 'sec' }
]

const riskFields: NumericField[] = [
  { key: 'min_kelly_size', label: 'Minimum Kelly gate', min: 0, max: 1, step: 0.005 },
  { key: 'min_bet_price', label: 'Minimum entry price', min: 0, max: 1, step: 0.01 },
  { key: 'stop_loss_pct', label: 'Stop loss', min: 0, step: 0.05 },
  { key: 'exit_days_before_expiry', label: 'Exit days before expiry', min: 0, step: 1, suffix: 'days' },
  { key: 'slippage_pct', label: 'Slippage assumption', min: 0, max: 1, step: 0.005 }
]

const wsFields: NumericField[] = [
  { key: 'max_ws_bets_per_day', label: 'Max WS bets per day', min: 0, step: 1 },
  { key: 'ws_bet_cooldown_secs', label: 'WS bet cooldown', min: 0, step: 30, suffix: 'sec' },
  { key: 'alert_throttle_mins', label: 'WS alert throttle', min: 0, step: 1, suffix: 'min' },
  { key: 'price_alert_cooldown_secs', label: 'Price alert cooldown', min: 0, step: 60, suffix: 'sec' }
]

const defaultFormConfig: GlobalConfigType = {
  config_poll_interval_secs: 60
}

// Presets only touch live-reloadable risk fields. Position sizing per strategy
// (kelly_fraction, min_bet, ...) is tuned in the strategy editor, not here.
const riskProfiles: Array<{ key: RiskProfile; label: string; patch: GlobalConfigType }> = [
  {
    key: 'conservative',
    label: 'Conservative',
    patch: {
      risk_profile: 'conservative',
      min_kelly_size: 0.03,
      min_bet_price: 0.2,
      stop_loss_pct: 0.25,
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
      min_kelly_size: 0.02,
      min_bet_price: 0.15,
      stop_loss_pct: 0.5,
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
      min_kelly_size: 0.01,
      min_bet_price: 0.1,
      stop_loss_pct: 0.75,
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
        <SectionHeader icon={Clock} title="Loop Intervals" subtitle="Scan cadence — live-reloaded via runtime-config polling" />

        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          {intervalFields.map((field) => (
            <NumberField key={field.key} field={field} value={Number(form[field.key] ?? 0)} onChange={(value) => setField(field.key, value)} />
          ))}
        </div>
      </section>

      <section className="cyber-card p-4">
        <SectionHeader icon={ShieldAlert} title="Risk & Sizing Gates" subtitle="Entry gates, stop loss, and early exit" />

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
          <RiskReadout label="Min Kelly gate" value={riskDisplay('min_kelly_size')} tone="green" />
          <RiskReadout label="Min entry price" value={riskDisplay('min_bet_price')} tone="cyan" />
        </div>

        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {riskFields.map((field) => (
            <NumberField key={field.key} field={field} value={Number(form[field.key] ?? 0)} onChange={(value) => setRiskField(field.key, value)} />
          ))}
        </div>

        <div className="mt-4 rounded-md border border-cyber-cyan/30 bg-cyber-cyan/10 p-3 text-sm text-cyber-cyan">
          Per-strategy sizing (Kelly fraction, min edge, min bet) is tuned in the Strategies tab. Model, news, and fee
          settings are environment-only and live in the bot's <code>.env</code>.
        </div>
      </section>

      <section className="cyber-card p-4">
        <SectionHeader icon={RadioTower} title="WebSocket Alerts" subtitle="Live-trade alert throttling and exposure caps" />

        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          {wsFields.map((field) => (
            <NumberField key={field.key} field={field} value={Number(form[field.key] ?? 0)} onChange={(value) => setField(field.key, value)} />
          ))}
        </div>
      </section>

      <div className="mt-2 flex justify-end">
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
