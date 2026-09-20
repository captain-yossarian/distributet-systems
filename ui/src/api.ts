import type {
  BroadcastResult,
  ContainerInfo,
  LogsResponse,
  SecondaryNode,
  SecondarySettings,
} from './types'

const MASTER_URL = import.meta.env.VITE_MASTER_URL ?? 'http://localhost:3000'

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw new Error(`request failed: ${res.status}`)
  }
  return res.json() as Promise<T>
}

export function fetchSecondaries(): Promise<SecondaryNode[]> {
  return fetch(`${MASTER_URL}/secondaries`).then((res) => json<SecondaryNode[]>(res))
}

export function stopSecondary(id: string): Promise<void> {
  return fetch(`${MASTER_URL}/secondaries/${encodeURIComponent(id)}/stop`, {
    method: 'POST',
  }).then((res) => {
    if (!res.ok) throw new Error(`request failed: ${res.status}`)
  })
}

export function startSecondary(id: string): Promise<void> {
  return fetch(`${MASTER_URL}/secondaries/${encodeURIComponent(id)}/start`, {
    method: 'POST',
  }).then((res) => {
    if (!res.ok) throw new Error(`request failed: ${res.status}`)
  })
}

export function fetchLogs(): Promise<LogsResponse> {
  return fetch(`${MASTER_URL}/logs`).then((res) => json<LogsResponse>(res))
}

export function postMessage(message: string, writeConcern: number): Promise<BroadcastResult> {
  return fetch(`${MASTER_URL}/post_message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ message, write_concern: writeConcern }),
  }).then((res) => json<BroadcastResult>(res))
}

export function spawnSecondary(): Promise<void> {
  return fetch(`${MASTER_URL}/secondaries/spawn`, { method: 'POST' }).then((res) => {
    if (!res.ok) throw new Error(`request failed: ${res.status}`)
  })
}

export function fetchContainers(): Promise<ContainerInfo[]> {
  return fetch(`${MASTER_URL}/docker/ps`).then((res) => json<ContainerInfo[]>(res))
}

export function fetchSecondarySettings(id: string): Promise<SecondarySettings> {
  return fetch(`${MASTER_URL}/secondaries/${encodeURIComponent(id)}/settings`).then((res) =>
    json<SecondarySettings>(res),
  )
}

export function updateSecondarySettings(
  id: string,
  settings: SecondarySettings,
): Promise<SecondarySettings> {
  return fetch(`${MASTER_URL}/secondaries/${encodeURIComponent(id)}/settings`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(settings),
  }).then((res) => json<SecondarySettings>(res))
}
