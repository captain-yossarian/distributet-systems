import { useEffect, useState, type FormEvent } from 'react'
import type { SecondarySettings } from './types'

export default function SecondaryControls({
  settings,
  onChange,
}: {
  settings: SecondarySettings | null
  onChange: (next: SecondarySettings) => Promise<void>
}) {
  const [nameInput, setNameInput] = useState('')
  const [delayInput, setDelayInput] = useState('0')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (settings) {
      setNameInput(settings.name)
      setDelayInput(String(settings.delay_ms))
    }
  }, [settings])

  if (!settings) {
    return <p className="monitor-controls-status">loading settings…</p>
  }

  const apply = async (next: SecondarySettings) => {
    setSaving(true)
    try {
      await onChange(next)
    } finally {
      setSaving(false)
    }
  }

  const save = (event: FormEvent) => {
    event.preventDefault()
    apply({
      name: nameInput.trim(),
      delay_ms: Math.max(0, Number(delayInput) || 0),
      failing: settings.failing,
    })
  }

  return (
    <form className="monitor-controls" onSubmit={save}>
      <h4>Settings</h4>
      <label className="control-field">
        <span>Name</span>
        <input
          type="text"
          value={nameInput}
          onChange={(event) => setNameInput(event.target.value)}
          placeholder={settings.name ? undefined : 'unnamed'}
          disabled={saving}
        />
      </label>
      <label className="control-field">
        <span>Delay (ms)</span>
        <input
          type="number"
          min={0}
          value={delayInput}
          onChange={(event) => setDelayInput(event.target.value)}
          disabled={saving}
        />
      </label>
      <button type="submit" className="controls-save" disabled={saving}>
        Save
      </button>
      <p className={`status-line${settings.failing ? ' down' : ''}`}>
        <span className={`status-dot${settings.failing ? ' offline' : ' online'}`} />
        {settings.failing ? 'Simulated down' : 'Online'}
      </p>
      <button
        type="button"
        className={`fail-toggle${settings.failing ? ' active' : ''}`}
        disabled={saving}
        onClick={() =>
          apply({ name: settings.name, delay_ms: settings.delay_ms, failing: !settings.failing })
        }
      >
        {settings.failing ? 'Restore work' : 'Simulate down'}
      </button>
    </form>
  )
}
