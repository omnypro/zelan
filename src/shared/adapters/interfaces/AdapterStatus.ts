/**
 * Adapter connection status
 */
export enum AdapterStatus {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  RECONNECTING = 'reconnecting',
  ERROR = 'error'
}

/**
 * Status with additional information
 */
export interface AdapterStatusInfo {
  status: AdapterStatus;
  message?: string;
  error?: Error;
  timestamp: number;
}