import { useState, type FormEvent } from 'react'
import AckCounter from './AckCounter'
import type { BroadcastResult } from './types'

export default function MessageForm({
  onSend,
  lastResult,
}: {
  onSend: (message: string) => Promise<void>
  lastResult: BroadcastResult | null
}) {
  const [message, setMessage] = useState('')
  const [sending, setSending] = useState(false)

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    if (!message.trim()) return
    setSending(true)
    try {
      await onSend(message)
      setMessage('')
    } finally {
      setSending(false)
    }
  }

  return (
    <section>
      <h2>Send message</h2>
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
      {lastResult && (
        <div className="result">
          <AckCounter
            sent={lastResult.delivered.length + lastResult.failed.length}
            acked={lastResult.delivered.length}
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
