// Adapter types
export type AdapterStatus = 'connected' | 'disconnected' | 'error' | 'disabled' | 'connecting'

export interface Adapter {
  id: string
  name: string
  description: string
  status: AdapterStatus
  statusDetails?: string
  icon?: string
}

// Event types
export interface StreamEvent {
  id: string
  source: string
  event_type: string
  payload: Record<string, any>
  timestamp: string
}

// WebSocket types
export interface WebSocketInfo {
  port: number
  active: boolean
  connections: number
  maxConnections: number
}

// Event bus stats
export interface EventBusStats {
  totalEvents: number
  activeSubscribers: number
  eventsPerSecond: number
  eventsBySource: Record<string, number>
}

// Form field types
export type FieldType = 'text' | 'number' | 'password' | 'checkbox' | 'textarea' | 'select'

export interface FieldOption {
  label: string
  value: string
}

export interface FormField {
  name: string
  label: string
  type: FieldType
  defaultValue: string | number | boolean
  placeholder?: string
  options?: FieldOption[]
  help?: string
}

// Configuration types
export interface AdapterConfig {
  enabled: boolean
  config: Record<string, any>
}

export interface WebSocketConfig {
  port: number
  maxConnections: number
  timeout: number
}