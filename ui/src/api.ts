import type { BroadcastResult, LogsResponse, SecondaryInfo, SecondarySettings } from './types'

const MASTER_URL = import.meta.env.VITE_MASTER_URL ?? 'http://localhost:3000'

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw new Error(`request failed: ${res.status}`)
  }
  return res.json() as Promise<T>
}

export function fetchSecondaries(): Promise<SecondaryInfo[]> {
  return fetch(`${MASTER_URL}/secondaries`).then((res) => json<SecondaryInfo[]>(res))
}

export function fetchLogs(): Promise<LogsResponse> {
  return fetch(`${MASTER_URL}/logs`).then((res) => json<LogsResponse>(res))
}

export function postMessage(message: string): Promise<BroadcastResult> {
  return fetch(`${MASTER_URL}/post_message`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ message }),
  }).then((res) => json<BroadcastResult>(res))
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
