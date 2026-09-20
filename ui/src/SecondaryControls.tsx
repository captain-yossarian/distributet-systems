import { useEffect, useRef, useState, type FormEvent } from 'react'
import type { SecondarySettings } from './types'

export default function SecondaryControls({
  settings,
  onChangeSettings,
  running,
  onStop,
  onStart,
}: {
  settings: SecondarySettings | null
  onChangeSettings: (next: SecondarySettings) => Promise<void>
  running: boolean
  onStop: () => Promise<void>
  onStart: () => Promise<void>
}) {
  const [nameInput, setNameInput] = useState('')
  const [delayInput, setDelayInput] = useState('0')
  const [savingSettings, setSavingSettings] = useState(false)
  const [powerBusy, setPowerBusy] = useState(false)

  // Hydrate the inputs from the server exactly once, the first time real
  // settings arrive. The app polls every few seconds, which would otherwise
  // re-fire this on every refresh and wipe out whatever the user is
  // currently typing before they get a chance to hit Save.
  const hydrated = useRef(false)
  useEffect(() => {
    if (settings && !hydrated.current) {
      setNameInput(settings.name)
      setDelayInput(String(settings.delay_ms))
      hydrated.current = true
    }
  }, [settings])

  const saveSettings = async (event: FormEvent) => {
    event.preventDefault()
    setSavingSettings(true)
    try {
      await onChangeSettings({
        name: nameInput.trim(),
        delay_ms: Math.max(0, Number(delayInput) || 0),
      })
    } finally {
      setSavingSettings(false)
    }
  }

  const togglePower = async () => {
    setPowerBusy(true)
    try {
      await (running ? onStop() : onStart())
    } finally {
      setPowerBusy(false)
    }
  }

  return (
    <div className="monitor-controls">
      <h4>Settings</h4>
      {settings ? (
        <form onSubmit={saveSettings}>
          <label className="control-field">
            <span>Name</span>
            <input
              type="text"
              value={nameInput}
              onChange={(event) => setNameInput(event.target.value)}
              placeholder={settings.name ? undefined : 'unnamed'}
              disabled={savingSettings}
            />
          </label>
          <label className="control-field">
            <span>Delay (ms)</span>
            <input
              type="number"
              min={0}
              value={delayInput}
              onChange={(event) => setDelayInput(event.target.value)}
              disabled={savingSettings}
            />
          </label>
          <button type="submit" className="controls-save" disabled={savingSettings}>
            Save
          </button>
        </form>
      ) : (
        <p className="monitor-controls-status">
          {running ? 'loading settings…' : 'settings unavailable while stopped'}
        </p>
      )}
      <p className={`status-line${running ? '' : ' down'}`}>
        <span className={`status-dot${running ? ' online' : ' offline'}`} />
        {running ? 'Running' : 'Stopped'}
      </p>
      <button
        type="button"
        className={`power-toggle${running ? '' : ' active'}`}
        disabled={powerBusy}
        onClick={togglePower}
      >
        {powerBusy
          ? running
            ? 'Stopping…'
            : 'Starting…'
          : running
            ? 'Stop server'
            : 'Start server'}
      </button>
    </div>
  )
}
