/** A secondary as shown to the UI: Docker's view (does it exist, is it
 * running) merged with the live registry's view (its reachable address,
 * only present while running and heartbeating). A stopped secondary still
 * appears with `running: false` and `address: null`. */
export interface SecondaryNode {
  id: string
  address: string | null
  running: boolean
}

export interface LoggedMessage {
  message: string
  timestamp: string
}

export interface BroadcastResult {
  message: string
  timestamp: string
  delivered: string[]
  failed: string[]
}

export interface MasterLogEntry {
  message: string
  timestamp: string
  sent: number
  acked: number
}

/** One retry attempt master made trying to deliver a message to this
 * secondary. `attempt` counts the retry itself (1st retry, 2nd retry, ...),
 * not the original attempt. */
export interface RetryEntry {
  message: string
  timestamp: string
  attempt: number
  max_attempts: number
}

export interface SecondaryLog {
  id: string
  messages: LoggedMessage[]
  retries: RetryEntry[]
}

export interface LogsResponse {
  master: MasterLogEntry[]
  /** Ordered by registration order (oldest first). */
  secondaries: SecondaryLog[]
}

export interface SecondarySettings {
  name: string
  delay_ms: number
}

export interface ContainerInfo {
  id: string
  name: string
  image: string
  state: string
  status: string
}
