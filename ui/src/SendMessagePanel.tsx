import { useEffect, useState, type FormEvent } from 'react'
import AckCounter from './AckCounter'
import type { BroadcastResult } from './types'

function writeConcernHint(w: number, totalNodes: number): string {
  if (w <= 1) return 'master only'
  if (w >= totalNodes) return 'master + all secondaries'
  return `master + ${w - 1} secondar${w - 1 === 1 ? 'y' : 'ies'}`
}

export default function SendMessagePanel({
  onSend,
  lastResult,
  secondaryCount,
}: {
  onSend: (message: string, writeConcern: number) => Promise<void>
  lastResult: BroadcastResult | null
  secondaryCount: number
}) {
  const totalNodes = 1 + secondaryCount
  const [message, setMessage] = useState('')
  const [writeConcern, setWriteConcern] = useState(1)
  const [sending, setSending] = useState(false)

  useEffect(() => {
    if (writeConcern > totalNodes) setWriteConcern(totalNodes)
  }, [totalNodes, writeConcern])

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    if (!message.trim()) return
    setSending(true)
    try {
      await onSend(message, writeConcern)
      setMessage('')
    } finally {
      setSending(false)
    }
  }

  return (
    <section className="panel">
      <div className="panel-header">
        <h2>Send message</h2>
      </div>
      <div className="write-concern-row">
        <span className="write-concern-label">Write concern</span>
        <div className="write-concern-buttons">
          {Array.from({ length: totalNodes }, (_, i) => i + 1).map((w) => (
            <button
              key={w}
              type="button"
              className={`wc-button${w === writeConcern ? ' active' : ''}`}
              onClick={() => setWriteConcern(w)}
              disabled={sending}
            >
              {w}
            </button>
          ))}
        </div>
        <span className="write-concern-hint">{writeConcernHint(writeConcern, totalNodes)}</span>
      </div>
      <form onSubmit={submit} className="message-form">
        <input
          value={message}
          onChange={(event) => setMessage(event.target.value)}
          placeholder="Broadcast a message to all secondaries…"
          disabled={sending}
        />
        <button type="submit" disabled={sending || !message.trim()}>
          {sending ? 'Sending…' : 'Send'}
        </button>
      </form>
      <p className="send-hint">
        send waits for {writeConcern} of {totalNodes} acknowledgements
      </p>
      {lastResult && (
        <div className="result">
          <AckCounter
            sent={1 + lastResult.delivered.length + lastResult.failed.length}
            acked={1 + lastResult.delivered.length}
          />
          <span>
            ACKed
            {lastResult.failed.length > 0 && ` — no ACK from: ${lastResult.failed.join(', ')}`}
          </span>
        </div>
      )}
    </section>
  )
}
