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

export interface SecondaryLog {
  id: string
  messages: LoggedMessage[]
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
