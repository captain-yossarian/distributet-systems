import { useState } from 'react'
import AckCounter from './AckCounter'
import { colorForId } from './colors'
import { formatTimestamp } from './format'
import type { LogsResponse, SecondaryLog, SecondarySettings } from './types'

function MasterColumn({ entries }: { entries: LogsResponse['master'] }) {
  return (
    <div className="log-column">
      <div className="log-column-header">
        <span className="log-column-title master-title">master</span>
      </div>
      <div className="log-column-list">
        {entries.length === 0 && <p className="empty">no messages yet</p>}
        {entries.map((entry, index) => (
          <div className="log-entry" key={index}>
            <div className="log-entry-top">
              <span className="timestamp" title={entry.timestamp}>
                {formatTimestamp(entry.timestamp)}
              </span>
              <AckCounter sent={entry.sent} acked={entry.acked} />
            </div>
            <p className="log-entry-message">{entry.message}</p>
          </div>
        ))}
      </div>
    </div>
  )
}

interface SecondaryEvent {
  key: string
  timestamp: string
  message: string
  kind: 'received' | 'retry'
  attempt?: number
  maxAttempts?: number
}

function buildEvents(log: SecondaryLog, clearedAt: string | null): SecondaryEvent[] {
  const events: SecondaryEvent[] = [
    ...log.messages.map((entry, index) => ({
      key: `recv-${index}-${entry.timestamp}`,
      timestamp: entry.timestamp,
      message: entry.message,
      kind: 'received' as const,
    })),
    ...log.retries.map((entry, index) => ({
      key: `retry-${index}-${entry.timestamp}-${entry.attempt}`,
      timestamp: entry.timestamp,
      message: entry.message,
      kind: 'retry' as const,
      attempt: entry.attempt,
      maxAttempts: entry.max_attempts,
    })),
  ]
  return events
    .filter((event) => !clearedAt || event.timestamp > clearedAt)
    .sort((a, b) => (a.timestamp < b.timestamp ? 1 : a.timestamp > b.timestamp ? -1 : 0))
}

function SecondaryColumn({
  id,
  label,
  log,
  clearedAt,
}: {
  id: string
  label: string
  log: SecondaryLog
  clearedAt: string | null
}) {
  const color = colorForId(id)
  const events = buildEvents(log, clearedAt)
  return (
    <div className="log-column">
      <div className="log-column-header">
        <span className="log-column-title" style={{ color: color.text }} title={id}>
          {label}
        </span>
      </div>
      <div className="log-column-list">
        {events.length === 0 && <p className="empty">no messages yet</p>}
        {events.map((event) => (
          <div className={`log-entry${event.kind === 'retry' ? ' retry' : ''}`} key={event.key}>
            <div className="log-entry-top">
              <span className="timestamp" title={event.timestamp}>
                {formatTimestamp(event.timestamp)}
              </span>
              {event.kind === 'retry' ? (
                <span className="log-retry">
                  retry {event.attempt}/{event.maxAttempts}
                </span>
              ) : (
                <span className="log-recv">recv</span>
              )}
            </div>
            <p className="log-entry-message">{event.message}</p>
          </div>
        ))}
      </div>
    </div>
  )
}

export default function LogsPanel({
  logs,
  secondarySettings,
}: {
  logs: LogsResponse
  secondarySettings: Record<string, SecondarySettings>
}) {
  const [clearedAt, setClearedAt] = useState<string | null>(null)

  // most recent first, regardless of the chronological (oldest-first) order
  // the API returns them in
  const filterMasterEntries = <T extends { timestamp: string }>(entries: T[]) =>
    (clearedAt ? entries.filter((entry) => entry.timestamp > clearedAt) : entries)
      .slice()
      .reverse()

  const sortedTimestamps = [
    ...logs.master.map((entry) => entry.timestamp),
    ...logs.secondaries.flatMap((secondary) => [
      ...secondary.messages.map((entry) => entry.timestamp),
      ...secondary.retries.map((entry) => entry.timestamp),
    ]),
  ].sort()
  const latestTimestamp = sortedTimestamps[sortedTimestamps.length - 1] ?? null

  return (
    <section className="panel logs-panel">
      <div className="panel-header">
        <h2>Logs</h2>
        <button
          type="button"
          className="clear-button"
          onClick={() => setClearedAt(latestTimestamp ?? new Date().toISOString())}
        >
          clear
        </button>
      </div>
      <div className="log-columns">
        <MasterColumn entries={filterMasterEntries(logs.master)} />
        {logs.secondaries.map((secondary) => {
          const name = secondarySettings[secondary.id]?.name.trim()
          return (
            <SecondaryColumn
              key={secondary.id}
              id={secondary.id}
              label={name || secondary.id}
              log={secondary}
              clearedAt={clearedAt}
            />
          )
        })}
      </div>
    </section>
  )
}
