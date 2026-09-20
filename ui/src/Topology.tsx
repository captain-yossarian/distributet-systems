import { useEffect, useRef, useState } from 'react'
import AckCounter from './AckCounter'
import { formatTimestamp } from './format'
import SecondaryControls from './SecondaryControls'
import type { LogsResponse, SecondaryInfo, SecondarySettings } from './types'

function MasterNode({ log }: { log: LogsResponse['master'] }) {
  const last = log[log.length - 1]
  return (
    <div className="node master-node">
      <div className="node-header">
        <span className="status-dot online" />
        <span className="node-title">master</span>
      </div>
      <div className="node-body">
        {last ? (
          <>
            <p className="last-message">{last.message}</p>
            <p className="node-meta">
              <span className="timestamp" title={last.timestamp}>
                {formatTimestamp(last.timestamp)}
              </span>
              <AckCounter sent={last.sent} acked={last.acked} />
            </p>
          </>
        ) : (
          <p className="node-meta empty">no messages sent yet</p>
        )}
      </div>
    </div>
  )
}

function Monitor({
  secondary,
  messages,
  settings,
  onUpdateSettings,
}: {
  secondary: SecondaryInfo
  messages: LogsResponse['secondaries'][number]['messages']
  settings: SecondarySettings | undefined
  onUpdateSettings: (id: string, next: SecondarySettings) => Promise<void>
}) {
  const last = messages[messages.length - 1]
  const down = settings?.failing ?? false
  const label = settings?.name.trim() ? settings.name : secondary.id
  const delayMs = settings?.delay_ms ?? 0

  const [settingsOpen, setSettingsOpen] = useState(false)
  const screenRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!settingsOpen) return
    const handleClickOutside = (event: MouseEvent) => {
      if (screenRef.current && !screenRef.current.contains(event.target as Node)) {
        setSettingsOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [settingsOpen])

  return (
    <div className="monitor-branch">
      <div className="branch-line" />
      <div className="monitor">
        <div className={`monitor-screen${down ? ' down' : ''}`} ref={screenRef}>
          <button
            type="button"
            className="settings-button"
            aria-label="secondary settings"
            onClick={() => setSettingsOpen((value) => !value)}
          >
            ⚙
          </button>
          <div className="monitor-screen-header">
            <span className={`status-dot${down ? ' offline' : ' online'}`} />
            <span className="monitor-id" title={secondary.id}>
              {label}
            </span>
          </div>
          <div className="monitor-content">
            {down ? (
              <p className="node-meta empty">simulated down — rejecting messages</p>
            ) : last ? (
              <>
                <p className="last-message">{last.message}</p>
                <p className="timestamp" title={last.timestamp}>
                  {formatTimestamp(last.timestamp)}
                </p>
              </>
            ) : (
              <p className="node-meta empty">waiting for messages…</p>
            )}
          </div>
          <div className="monitor-footer">
            {messages.length} received
            {delayMs > 0 && <span className="delay-badge"> · {delayMs}ms delay</span>}
          </div>
          {settingsOpen && (
            <div className="settings-popover">
              <SecondaryControls
                settings={settings ?? null}
                onChange={(next) => onUpdateSettings(secondary.id, next)}
              />
            </div>
          )}
        </div>
        <div className="monitor-stand" />
        <div className="monitor-base" />
        <div className="monitor-address">{secondary.address}</div>
      </div>
    </div>
  )
}

export default function Topology({
  secondaries,
  logs,
  secondarySettings,
  onUpdateSettings,
}: {
  secondaries: SecondaryInfo[]
  logs: LogsResponse
  secondarySettings: Record<string, SecondarySettings>
  onUpdateSettings: (id: string, next: SecondarySettings) => Promise<void>
}) {
  return (
    <section className="topology">
      <h2>Cluster</h2>
      <MasterNode log={logs.master} />
      {secondaries.length > 0 && <div className="topology-connector" />}
      <div className="monitor-row">
        {secondaries.length === 0 && <p className="empty">no secondaries registered</p>}
        {secondaries.map((secondary) => (
          <Monitor
            key={secondary.id}
            secondary={secondary}
            messages={logs.secondaries.find((entry) => entry.id === secondary.id)?.messages ?? []}
            settings={secondarySettings[secondary.id]}
            onUpdateSettings={onUpdateSettings}
          />
        ))}
      </div>
    </section>
  )
}
