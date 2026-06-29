import { Save, X } from 'lucide-react'
import type { FormEvent } from 'react'
import { useEffect, useState } from 'react'
import type { Strategy, StrategyPatch } from '../../types'

type StrategyEditorProps = {
  strategy: Strategy | null
  saving: boolean
  onClose: () => void
  onSave: (name: string, patch: StrategyPatch) => Promise<void>
}

export function StrategyEditor({ strategy, saving, onClose, onSave }: StrategyEditorProps) {
  const [form, setForm] = useState<StrategyPatch>({})

  useEffect(() => {
    setForm(strategy ? { ...strategy } : {})
  }, [strategy])

  if (!strategy) {
    return null
  }

  const setNumber = (key: keyof StrategyPatch, value: number) => {
    setForm((current) => ({ ...current, [key]: value }))
  }

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    await onSave(strategy.name, form)
  }

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4 backdrop-blur-sm">
      <form onSubmit={submit} className="cyber-card w-full max-w-2xl p-5">
        <div className="mb-5 flex items-center justify-between gap-4">
          <div>
            <p className="text-xs uppercase text-cyber-muted">Runtime tuning</p>
            <h3 className="font-orbitron text-2xl text-cyber-cyan">{strategy.name}</h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="grid h-10 w-10 place-items-center rounded-md border border-cyber-border text-cyber-muted hover:border-cyber-red/50 hover:text-cyber-red"
            title="Close"
          >
            <X size={18} />
          </button>
        </div>

        <div className="grid gap-5 md:grid-cols-2">
          <SliderField label="Min edge" value={form.min_edge ?? 0} min={0} max={0.3} step={0.01} onChange={(value) => setNumber('min_edge', value)} />
          <SliderField
            label="Min confidence"
            value={form.min_confidence ?? 0}
            min={0}
            max={1}
            step={0.01}
            onChange={(value) => setNumber('min_confidence', value)}
          />
          <SliderField
            label="Kelly fraction"
            value={form.kelly_fraction ?? 0}
            min={0}
            max={1}
            step={0.01}
            onChange={(value) => setNumber('kelly_fraction', value)}
          />
          <NumberField
            label="Max signals per day"
            value={form.max_signals_per_day ?? 0}
            min={0}
            step={1}
            onChange={(value) => setNumber('max_signals_per_day', value)}
          />
          <NumberField label="Min bet" value={form.min_bet ?? 0} min={0} step={1} onChange={(value) => setNumber('min_bet', value)} />
        </div>

        <div className="mt-6 flex justify-end">
          <button
            type="submit"
            disabled={saving}
            className="inline-flex items-center gap-2 rounded-md border border-cyber-green/40 bg-cyber-green/10 px-4 py-3 text-sm text-cyber-green hover:bg-cyber-green/20 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <Save size={16} />
            {saving ? 'Saving' : 'Apply'}
          </button>
        </div>
      </form>
    </div>
  )
}

function SliderField({
  label,
  value,
  min,
  max,
  step,
  onChange
}: {
  label: string
  value: number
  min: number
  max: number
  step: number
  onChange: (value: number) => void
}) {
  return (
    <label className="grid gap-2 text-sm text-cyber-muted">
      <span className="flex items-center justify-between">
        {label}
        <span className="font-orbitron text-cyber-text">{value.toFixed(2)}</span>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="accent-cyber-cyan"
      />
    </label>
  )
}

function NumberField({
  label,
  value,
  min,
  step,
  onChange
}: {
  label: string
  value: number
  min: number
  step: number
  onChange: (value: number) => void
}) {
  return (
    <label className="grid gap-2 text-sm text-cyber-muted">
      {label}
      <input
        type="number"
        min={min}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="cyber-input px-3 py-3"
      />
    </label>
  )
}
