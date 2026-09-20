export interface SecondaryInfo {
  id: string
  address: string
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
  failing: boolean
}
