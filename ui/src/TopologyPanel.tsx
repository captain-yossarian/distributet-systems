import { useEffect, useRef, useState } from 'react'
import AckCounter from './AckCounter'
import { colorForId } from './colors'
import { formatTimestamp } from './format'
import SecondaryControls from './SecondaryControls'
import type { LogsResponse, SecondaryNode, SecondarySettings } from './types'

function MasterCard({ log }: { log: LogsResponse['master'] }) {
  const last = log[log.length - 1]
  return (
    <div className="node-card master-card">
      <div className="node-card-header">
        <span className="node-card-title">
          <span className="status-dot online" />
          MASTER
        </span>
        <span className="node-card-port">:3000</span>
      </div>
      {last ? (
        <>
          <p className="node-card-message">{last.message}</p>
          <div className="node-card-footer">
            <span className="timestamp" title={last.timestamp}>
              {formatTimestamp(last.timestamp)}
            </span>
            <AckCounter sent={last.sent} acked={last.acked} />
          </div>
        </>
      ) : (
        <p className="node-card-empty">no messages sent yet</p>
      )}
    </div>
  )
}

function SecondaryCard({
  secondary,
  messages,
  settings,
  onUpdateSettings,
  onStop,
  onStart,
}: {
  secondary: SecondaryNode
  messages: LogsResponse['secondaries'][number]['messages']
  settings: SecondarySettings | undefined
  onUpdateSettings: (id: string, next: SecondarySettings) => Promise<void>
  onStop: (id: string) => Promise<void>
  onStart: (id: string) => Promise<void>
}) {
  const last = messages[messages.length - 1]
  const down = !secondary.running
  const label = settings?.name.trim() ? settings.name : secondary.id
  const delayMs = settings?.delay_ms ?? 0
  const color = colorForId(secondary.id)

  const [settingsOpen, setSettingsOpen] = useState(false)
  const cardRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!settingsOpen) return
    const handleClickOutside = (event: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(event.target as Node)) {
        setSettingsOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [settingsOpen])

  return (
    <div className={`node-card secondary-card${down ? ' down' : ''}`} ref={cardRef}>
      <button
        type="button"
        className="settings-button"
        aria-label="secondary settings"
        onClick={() => setSettingsOpen((value) => !value)}
      >
        ⚙
      </button>
      <div className="node-card-header">
        <span
          className="node-card-title"
          style={{ color: down ? undefined : color.text }}
          title={secondary.id}
        >
          <span className={`status-dot${down ? ' offline' : ' online'}`} />
          {label}
        </span>
      </div>
      {down ? (
        <p className="node-card-empty">stopped</p>
      ) : last ? (
        <p className="node-card-message">{last.message}</p>
      ) : (
        <p className="node-card-empty">waiting for messages…</p>
      )}
      <div className="node-card-divider" />
      <div className="node-card-footer">
        <span>
          {messages.length} received
          {delayMs > 0 && <span className="delay-badge"> · {delayMs}ms delay</span>}
        </span>
        <span className="node-card-address">{secondary.address ?? '—'}</span>
      </div>
      {settingsOpen && (
        <div className="settings-popover">
          <SecondaryControls
            settings={settings ?? null}
            onChangeSettings={(next) => onUpdateSettings(secondary.id, next)}
            running={secondary.running}
            onStop={() => onStop(secondary.id)}
            onStart={() => onStart(secondary.id)}
          />
        </div>
      )}
    </div>
  )
}

export default function TopologyPanel({
  secondaries,
  logs,
  secondarySettings,
  onUpdateSettings,
  onStop,
  onStart,
}: {
  secondaries: SecondaryNode[]
  logs: LogsResponse
  secondarySettings: Record<string, SecondarySettings>
  onUpdateSettings: (id: string, next: SecondarySettings) => Promise<void>
  onStop: (id: string) => Promise<void>
  onStart: (id: string) => Promise<void>
}) {
  return (
    <section className="panel">
      <div className="panel-header">
        <h2>Topology</h2>
        <span className="panel-header-meta">
          1 master · {secondaries.length} secondar{secondaries.length === 1 ? 'y' : 'ies'}
        </span>
      </div>
      <div className="topology-tree">
        <MasterCard log={logs.master} />
        {secondaries.length > 0 && <div className="topology-connector" />}
        <div className="node-row">
          {secondaries.length === 0 && <p className="empty">no secondaries registered</p>}
          {secondaries.map((secondary) => (
            <SecondaryCard
              key={secondary.id}
              secondary={secondary}
              messages={logs.secondaries.find((entry) => entry.id === secondary.id)?.messages ?? []}
              settings={secondarySettings[secondary.id]}
              onUpdateSettings={onUpdateSettings}
              onStop={onStop}
              onStart={onStart}
            />
          ))}
        </div>
      </div>
    </section>
  )
}
