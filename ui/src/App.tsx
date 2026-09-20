import { useCallback, useEffect, useState } from 'react'
import { fetchLogs, fetchSecondaries, postMessage } from './api'
import LogsView from './LogsView'
import MessageForm from './MessageForm'
import Topology from './Topology'
import type { BroadcastResult, LogsResponse, SecondaryInfo } from './types'
import './App.css'

const POLL_INTERVAL_MS = 3000

export default function App() {
  const [secondaries, setSecondaries] = useState<SecondaryInfo[]>([])
  const [logs, setLogs] = useState<LogsResponse>({ master: [], secondaries: [] })
  const [lastResult, setLastResult] = useState<BroadcastResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const [nextSecondaries, nextLogs] = await Promise.all([fetchSecondaries(), fetchLogs()])
      setSecondaries(nextSecondaries)
      setLogs(nextLogs)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed to reach master')
    }
  }, [])

  useEffect(() => {
    refresh()
    const interval = setInterval(refresh, POLL_INTERVAL_MS)
    return () => clearInterval(interval)
  }, [refresh])

  const handleSend = async (message: string) => {
    const result = await postMessage(message)
    setLastResult(result)
    await refresh()
  }

  return (
    <main className="app">
      <h1>Distributed Chat Console</h1>
      {error && <p className="error">{error}</p>}
      <Topology secondaries={secondaries} logs={logs} />
      <MessageForm onSend={handleSend} lastResult={lastResult} />
      <LogsView logs={logs} />
    </main>
  )
}
