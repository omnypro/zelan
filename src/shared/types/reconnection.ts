/**
 * Reconnection options for controlling adapter reconnection behavior
 */
export interface ReconnectionOptions {
  /** Maximum reconnection attempts before backing off to long interval */
  maxAttempts: number
  /** Initial delay before first retry (ms) */
  initialDelay: number
  /** Maximum delay between retries (ms) */
  maxDelay: number
  /** Long interval delay after max attempts (ms) */
  longIntervalDelay: number
  /** Whether to auto-reset attempt count after successful reconnection */
  resetCountOnSuccess: boolean
}
