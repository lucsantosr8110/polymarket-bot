import { Save } from 'lucide-react'
import type { FormEvent } from 'react'
import { useEffect, useState } from 'react'
import type { GlobalConfig as GlobalConfigType } from '../../types'

type GlobalConfigProps = {
  config: GlobalConfigType | null
  saving: boolean
  onSave: (patch: GlobalConfigType) => Promise<void>
}

const numericKeys = ['scan_interval_mins', 'bet_scan_interval_mins', 'heartbeat_interval_mins', 'min_volume', 'max_markets_fetch']

export function GlobalConfig({ config, saving, onSave }: GlobalConfigProps) {
  const [form, setForm] = useState<GlobalConfigType>({})

  useEffect(() => {
    setForm(config ?? {})
  }, [config])

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    await onSave(form)
  }

  return (
    <form onSubmit={submit} className="cyber-card p-4">
      <div className="mb-5">
        <h3 className="font-orbitron text-lg text-cyber-text">Global Runtime Config</h3>
        <p className="text-sm text-cyber-muted">Saved to FastAPI runtime_config</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {numericKeys.map((key) => (
          <label key={key} className="grid gap-2 text-sm text-cyber-muted">
            {key}
            <input
              type="number"
              value={Number(form[key] ?? 0)}
              onChange={(event) => setForm((current) => ({ ...current, [key]: Number(event.target.value) }))}
              className="cyber-input px-3 py-3"
            />
          </label>
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
}
