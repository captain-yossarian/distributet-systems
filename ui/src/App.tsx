import { useCallback, useEffect, useState } from 'react'
import {
  fetchContainers,
  fetchLogs,
  fetchSecondaries,
  fetchSecondarySettings,
  postMessage,
  spawnSecondary,
  startSecondary,
  stopSecondary,
  updateSecondarySettings,
} from './api'
import ContainersPanel from './ContainersPanel'
import Header from './Header'
import LogsPanel from './LogsPanel'
import SendMessagePanel from './SendMessagePanel'
import TopologyPanel from './TopologyPanel'
import type {
  BroadcastResult,
  ContainerInfo,
  LogsResponse,
  SecondaryNode,
  SecondarySettings,
} from './types'
import './App.css'

const POLL_INTERVAL_MS = 3000

export default function App() {
  const [secondaries, setSecondaries] = useState<SecondaryNode[]>([])
  const [logs, setLogs] = useState<LogsResponse>({ master: [], secondaries: [] })
  const [secondarySettings, setSecondarySettings] = useState<Record<string, SecondarySettings>>({})
  const [containers, setContainers] = useState<ContainerInfo[]>([])
  const [lastResult, setLastResult] = useState<BroadcastResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [spawning, setSpawning] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const [nextSecondaries, nextLogs, nextContainers] = await Promise.all([
        fetchSecondaries(),
        fetchLogs(),
        fetchContainers(),
      ])
      setSecondaries(nextSecondaries)
      setLogs(nextLogs)
      setContainers(nextContainers)
      setError(null)

      const settled = await Promise.all(
        nextSecondaries
          .filter((secondary) => secondary.running)
          .map(async (secondary) => {
            try {
              return [secondary.id, await fetchSecondarySettings(secondary.id)] as const
            } catch {
              return null
            }
          }),
      )
      setSecondarySettings((prev) => {
        const next = { ...prev }
        for (const entry of settled) {
          if (entry) next[entry[0]] = entry[1]
        }
        return next
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed to reach master')
    }
  }, [])

  useEffect(() => {
    refresh()
    const interval = setInterval(refresh, POLL_INTERVAL_MS)
    return () => clearInterval(interval)
  }, [refresh])

  const handleSend = async (message: string, writeConcern: number) => {
    const result = await postMessage(message, writeConcern)
    setLastResult(result)
    await refresh()
  }

  const handleUpdateSettings = async (id: string, next: SecondarySettings) => {
    const updated = await updateSecondarySettings(id, next)
    setSecondarySettings((prev) => ({ ...prev, [id]: updated }))
  }

  const handleAddSecondary = async () => {
    setSpawning(true)
    try {
      await spawnSecondary()
    } finally {
      setSpawning(false)
    }
  }

  const handleStop = async (id: string) => {
    await stopSecondary(id)
    await refresh()
  }

  const handleStart = async (id: string) => {
    await startSecondary(id)
    await refresh()
  }

  return (
    <main className="app">
      <Header
        healthyCount={1 + secondaries.filter((secondary) => secondary.running).length}
        totalCount={1 + secondaries.length}
        onAddSecondary={handleAddSecondary}
        spawning={spawning}
      />
      {error && <p className="error">{error}</p>}
      <div className="layout">
        <div className="layout-left">
          <TopologyPanel
            secondaries={secondaries}
            logs={logs}
            secondarySettings={secondarySettings}
            onUpdateSettings={handleUpdateSettings}
            onStop={handleStop}
            onStart={handleStart}
          />
          <SendMessagePanel
            onSend={handleSend}
            lastResult={lastResult}
            secondaryCount={secondaries.length}
          />
          <ContainersPanel containers={containers} />
        </div>
        <LogsPanel logs={logs} secondarySettings={secondarySettings} />
      </div>
    </main>
  )
}
