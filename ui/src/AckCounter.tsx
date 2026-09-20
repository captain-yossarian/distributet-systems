export default function AckCounter({ sent, acked }: { sent: number; acked: number }) {
  const complete = sent > 0 && acked === sent
  return (
    <span className={`ack-counter${complete ? ' complete' : ''}`}>
      {sent}/{acked}
    </span>
  )
}
