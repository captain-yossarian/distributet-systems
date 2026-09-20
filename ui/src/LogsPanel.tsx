import { useState } from 'react'
import AckCounter from './AckCounter'
import { colorForId } from './colors'
import { formatTimestamp } from './format'
import type { LogsResponse, SecondarySettings } from './types'

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

function SecondaryColumn({
  id,
  label,
  entries,
}: {
  id: string
  label: string
  entries: LogsResponse['secondaries'][number]['messages']
}) {
  const color = colorForId(id)
  return (
    <div className="log-column">
      <div className="log-column-header">
        <span className="log-column-title" style={{ color: color.text }} title={id}>
          {label}
        </span>
      </div>
      <div className="log-column-list">
        {entries.length === 0 && <p className="empty">no messages yet</p>}
        {entries.map((entry, index) => (
          <div className="log-entry" key={index}>
            <div className="log-entry-top">
              <span className="timestamp" title={entry.timestamp}>
                {formatTimestamp(entry.timestamp)}
              </span>
              <span className="log-recv">recv</span>
            </div>
            <p className="log-entry-message">{entry.message}</p>
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
  const filterEntries = <T extends { timestamp: string }>(entries: T[]) =>
    (clearedAt ? entries.filter((entry) => entry.timestamp > clearedAt) : entries)
      .slice()
      .reverse()

  const sortedTimestamps = [
    ...logs.master.map((entry) => entry.timestamp),
    ...logs.secondaries.flatMap((secondary) => secondary.messages.map((entry) => entry.timestamp)),
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
        <MasterColumn entries={filterEntries(logs.master)} />
        {logs.secondaries.map((secondary) => {
          const name = secondarySettings[secondary.id]?.name.trim()
          return (
            <SecondaryColumn
              key={secondary.id}
              id={secondary.id}
              label={name || secondary.id}
              entries={filterEntries(secondary.messages)}
            />
          )
        })}
      </div>
    </section>
  )
}
