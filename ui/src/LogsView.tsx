import AckCounter from './AckCounter'
import { formatTimestamp } from './format'
import type { LoggedMessage, LogsResponse, MasterLogEntry, SecondarySettings } from './types'

function MasterLogColumn({ entries }: { entries: MasterLogEntry[] }) {
  return (
    <div className="log-column">
      <h3>master</h3>
      <ul>
        {entries.length === 0 && <li className="empty">no messages yet</li>}
        {entries.map((entry, index) => (
          <li key={index}>
            <span className="timestamp" title={entry.timestamp}>
              {formatTimestamp(entry.timestamp)}
            </span>{' '}
            {entry.message} <AckCounter sent={entry.sent} acked={entry.acked} />
          </li>
        ))}
      </ul>
    </div>
  )
}

function SecondaryLogColumn({ title, entries }: { title: string; entries: LoggedMessage[] }) {
  return (
    <div className="log-column">
      <h3>{title}</h3>
      <ul>
        {entries.length === 0 && <li className="empty">no messages yet</li>}
        {entries.map((entry, index) => (
          <li key={index}>
            <span className="timestamp" title={entry.timestamp}>
              {formatTimestamp(entry.timestamp)}
            </span>{' '}
            {entry.message}
          </li>
        ))}
      </ul>
    </div>
  )
}

export default function LogsView({
  logs,
  secondarySettings,
}: {
  logs: LogsResponse
  secondarySettings: Record<string, SecondarySettings>
}) {
  return (
    <section>
      <h2>Logs</h2>
      <div className="logs">
        <MasterLogColumn entries={logs.master} />
        {logs.secondaries.map((secondary) => {
          const name = secondarySettings[secondary.id]?.name.trim()
          return (
            <SecondaryLogColumn
              key={secondary.id}
              title={name || secondary.id}
              entries={secondary.messages}
            />
          )
        })}
      </div>
    </section>
  )
}
