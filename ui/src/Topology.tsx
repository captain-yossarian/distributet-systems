import AckCounter from './AckCounter'
import { formatTimestamp } from './format'
import type { LogsResponse, SecondaryInfo } from './types'

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
}: {
  secondary: SecondaryInfo
  messages: LogsResponse['secondaries'][number]['messages']
}) {
  const last = messages[messages.length - 1]
  return (
    <div className="monitor-branch">
      <div className="branch-line" />
      <div className="monitor">
        <div className="monitor-screen">
          <div className="monitor-screen-header">
            <span className="status-dot online" />
            <span className="monitor-id">{secondary.id}</span>
          </div>
          <div className="monitor-content">
            {last ? (
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
          <div className="monitor-footer">{messages.length} received</div>
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
}: {
  secondaries: SecondaryInfo[]
  logs: LogsResponse
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
          />
        ))}
      </div>
    </section>
  )
}
