/**
 * Possible statuses for an adapter
 */
export enum AdapterStatus {
  /**
   * The adapter is disconnected and not attempting to connect
   */
  DISCONNECTED = 'disconnected',

  /**
   * The adapter is in the process of connecting
   */
  CONNECTING = 'connecting',

  /**
   * The adapter is connected and functioning
   */
  CONNECTED = 'connected',

  /**
   * The adapter is disconnected and actively attempting to reconnect
   */
  RECONNECTING = 'reconnecting',

  /**
   * The adapter has encountered an error
   */
  ERROR = 'error',

  /**
   * The adapter is not properly configured
   */
  MISCONFIGURED = 'misconfigured'
}

/**
 * Extended status information for an adapter
 */
export interface AdapterStatusInfo {
  /**
   * Current adapter status
   */
  status: AdapterStatus

  /**
   * Last time the status was updated (timestamp)
   */
  lastUpdated: number

  /**
   * Error message if status is ERROR
   */
  error?: string

  /**
   * Additional details about the current status
   */
  details?: Record<string, unknown>

  /**
   * Time the adapter was last connected (timestamp)
   */
  lastConnected?: number

  /**
   * Number of connection attempts if reconnecting
   */
  connectionAttempts?: number
}
