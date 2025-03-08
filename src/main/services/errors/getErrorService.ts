import { ErrorService } from './ErrorService'
import { EventBus } from '@s/core/bus'

let errorServiceInstance: ErrorService | null = null

/**
 * Get the global error service instance
 * @param eventBus The event bus to use for error reporting (required on first call)
 * @returns The global error service instance
 */
export function getErrorService(eventBus?: EventBus): ErrorService {
  if (!errorServiceInstance) {
    if (!eventBus) {
      throw new Error('eventBus must be provided when first initializing the ErrorService')
    }
    errorServiceInstance = ErrorService.getInstance(eventBus)
  }
  return errorServiceInstance
}

/**
 * Reset the global error service instance
 * Primarily used for testing
 */
export function resetErrorService(): void {
  if (errorServiceInstance) {
    errorServiceInstance.dispose()
  }
  errorServiceInstance = null
}