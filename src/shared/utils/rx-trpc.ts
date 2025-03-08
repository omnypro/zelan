import { Observable, EMPTY, catchError } from 'rxjs'
import { ServiceAdapter } from '@s/adapters/interfaces/ServiceAdapter'

/**
 * Standardized error type for cross-process serialization
 */
export interface SerializableError {
  message: string
  code: string
  data?: Record<string, unknown>
}

/**
 * Convert any error to a serializable format
 */
export function toSerializableError(error: unknown): SerializableError {
  if (error instanceof Error) {
    return {
      message: error.message,
      code: error.name || 'UNKNOWN_ERROR',
      data: { stack: error.stack }
    }
  }
  return {
    message: String(error),
    code: 'UNKNOWN_ERROR'
  }
}

/**
 * Make any object safely serializable for IPC
 */
export function toSerializable<T extends Record<string, any>>(obj: T): Record<string, unknown> {
  return JSON.parse(JSON.stringify(obj))
}

/**
 * Helper to create a subscription handler from an Observable
 * Standardizes error handling and subscription management
 */
export function createSubscriptionHandler<T>(
  observable$: Observable<T>,
  sendEvent: (data: T) => void,
  handleError: (err: Error) => void,
  handleComplete?: () => void
) {
  return observable$
    .pipe(
      // Add retry capability for transient errors
      catchError((err) => {
        handleError(err)
        // Return empty to continue instead of terminating
        return EMPTY
      })
    )
    .subscribe({
      next: (data) => sendEvent(data),
      error: (err) => handleError(err),
      complete: () => handleComplete?.()
    })
}

/**
 * Type guard for validating tRPC responses
 */
export function isValidTRPCResponse(data: unknown): data is { type: string } {
  return typeof data === 'object' && data !== null && 'type' in data
}

/**
 * Interface for serialized adapter format
 */
export interface SerializableAdapter {
  id: string
  name: string
  type: string
  status?: {
    status: string
    message?: string
    timestamp: number
  }
  enabled: boolean
  options?: Record<string, unknown>
}

/**
 * Creates a standardized adapter object that is safely serializable
 * for sending between processes
 */
export function createSerializableAdapter(adapter: ServiceAdapter): SerializableAdapter {
  // First convert to unknown to avoid direct conversion errors
  const serialized = toSerializable({
    id: adapter.id,
    name: adapter.name,
    type: adapter.type,
    // Use status$ value if available, else create a default status
    status: adapter.status$
      ? undefined
      : {
          status: 'unknown',
          timestamp: Date.now()
        },
    enabled: adapter.enabled,
    options: adapter.options ? toSerializable(adapter.options) : undefined
  })

  return serialized as unknown as SerializableAdapter
}
