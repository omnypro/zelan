import { EventBus } from '@s/core/bus'
import { createSystemEvent } from '@s/core/events'
import { SystemEventType } from '@s/types/events'
import { ApplicationError, ErrorCategory, ErrorSeverity, ErrorMetadata } from '@s/errors/ErrorTypes'
import { BehaviorSubject, Observable, Subject } from 'rxjs'
import { filter } from 'rxjs/operators'
import { SubscriptionManager } from '@s/utils/subscription-manager'
import { getLoggingService, ComponentLogger } from '@m/services/logging'

/**
 * Interface for error handlers
 */
export interface ErrorHandler {
  handleError(error: ApplicationError): void
}

/**
 * Service for centralized error handling
 */
export class ErrorService {
  private static instance: ErrorService
  private errorSubject = new Subject<ApplicationError>()
  private lastErrorSubject = new BehaviorSubject<ApplicationError | null>(null)
  private handlers: ErrorHandler[] = []
  private subscriptionManager = new SubscriptionManager()
  private logger: ComponentLogger

  constructor(private eventBus: EventBus) {
    // Initialize logger
    this.logger = getLoggingService().createLogger('ErrorService')

    // Subscribe to the error stream to update the last error
    this.subscriptionManager.add(
      this.errorSubject.subscribe((error) => {
        this.lastErrorSubject.next(error)
      })
    )
  }

  /**
   * Get the singleton instance
   */
  static getInstance(eventBus: EventBus): ErrorService {
    if (!ErrorService.instance) {
      ErrorService.instance = new ErrorService(eventBus)
    }
    return ErrorService.instance
  }

  /**
   * Add an error handler to the service
   */
  addHandler(handler: ErrorHandler): void {
    this.handlers.push(handler)
  }

  /**
   * Remove an error handler
   */
  removeHandler(handler: ErrorHandler): void {
    const index = this.handlers.indexOf(handler)
    if (index !== -1) {
      this.handlers.splice(index, 1)
    }
  }

  /**
   * Report an error to the service
   */
  reportError(error: Error | ApplicationError, metadata?: ErrorMetadata): void {
    // Convert to ApplicationError if needed
    const appError = this.normalizeError(error, metadata)

    // Log to console
    this.logToConsole(appError)

    // Publish to the error stream
    this.errorSubject.next(appError)

    // Publish to the event bus
    this.publishToEventBus(appError)

    // Notify all handlers
    this.notifyHandlers(appError)
  }

  /**
   * Get the error stream for observing all errors
   */
  errors$(): Observable<ApplicationError> {
    return this.errorSubject.asObservable()
  }

  /**
   * Get the latest error
   */
  getLastError(): ApplicationError | null {
    return this.lastErrorSubject.value
  }

  /**
   * Get an observable of the latest error
   */
  lastError$(): Observable<ApplicationError | null> {
    return this.lastErrorSubject.asObservable()
  }

  /**
   * Get errors filtered by category
   */
  getErrorsByCategory(category: ErrorCategory): Observable<ApplicationError> {
    return this.errorSubject.pipe(filter((error) => error.category === category))
  }

  /**
   * Get errors filtered by severity
   */
  getErrorsBySeverity(severity: ErrorSeverity): Observable<ApplicationError> {
    return this.errorSubject.pipe(filter((error) => error.severity === severity))
  }

  /**
   * Dispose of the error service
   */
  dispose(): void {
    this.subscriptionManager.unsubscribeAll()
    this.handlers = []
  }

  /**
   * Convert any error to an ApplicationError
   */
  private normalizeError(error: Error | ApplicationError, metadata?: ErrorMetadata): ApplicationError {
    // If it's already an ApplicationError, just return it
    if (error instanceof ApplicationError) {
      // Merge additional metadata if provided
      if (metadata) {
        return new ApplicationError(
          error.message,
          error.category,
          error.severity,
          { ...error.metadata, ...metadata },
          error.originalError || error
        )
      }
      return error
    }

    // Determine error category from error name or type
    let category = ErrorCategory.RUNTIME
    if (error.name === 'NetworkError' || error.message.includes('network') || error.message.includes('connection')) {
      category = ErrorCategory.NETWORK
    } else if (error.name === 'ValidationError' || error.message.includes('invalid')) {
      category = ErrorCategory.VALIDATION
    }

    // Create a new ApplicationError
    return new ApplicationError(error.message, category, ErrorSeverity.ERROR, metadata || {}, error)
  }

  /**
   * Log the error to the console
   */
  private logToConsole(error: ApplicationError): void {
    const { severity, category, message, metadata, stack } = error

    // Format the log message
    const metadataStr =
      metadata && Object.keys(metadata).length > 0 ? `\nMetadata: ${JSON.stringify(metadata, null, 2)}` : ''

    const logMsg = `[${severity.toUpperCase()}] [${category}] ${message}${metadataStr}`

    // Log with appropriate severity
    switch (severity) {
      case ErrorSeverity.DEBUG:
        this.logger.debug(logMsg, { stack })
        break
      case ErrorSeverity.INFO:
        this.logger.info(logMsg, { stack })
        break
      case ErrorSeverity.WARNING:
        this.logger.warn(logMsg, { stack })
        break
      case ErrorSeverity.ERROR:
      case ErrorSeverity.CRITICAL:
        this.logger.error(logMsg, { stack })
        break
    }
  }

  /**
   * Publish the error to the event bus
   */
  private publishToEventBus(error: ApplicationError): void {
    // Map severity to system event type
    let eventType: SystemEventType
    switch (error.severity) {
      case ErrorSeverity.DEBUG:
      case ErrorSeverity.INFO:
        eventType = SystemEventType.INFO
        break
      case ErrorSeverity.WARNING:
        eventType = SystemEventType.WARNING
        break
      case ErrorSeverity.ERROR:
      case ErrorSeverity.CRITICAL:
      default:
        eventType = SystemEventType.ERROR
        break
    }

    // Map severity to level for system event
    let level: 'info' | 'warning' | 'error'
    switch (error.severity) {
      case ErrorSeverity.DEBUG:
      case ErrorSeverity.INFO:
        level = 'info'
        break
      case ErrorSeverity.WARNING:
        level = 'warning'
        break
      case ErrorSeverity.ERROR:
      case ErrorSeverity.CRITICAL:
      default:
        level = 'error'
        break
    }

    // Publish the event with error details
    this.eventBus.publish(
      createSystemEvent(eventType, error.message, level, {
        errorService: true,
        category: error.category,
        severity: error.severity,
        timestamp: error.timestamp,
        metadata: error.metadata,
        stack: error.stack
      })
    )
  }

  /**
   * Notify all registered error handlers
   */
  private notifyHandlers(error: ApplicationError): void {
    for (const handler of this.handlers) {
      try {
        handler.handleError(error)
      } catch (handlerError) {
        // Don't let handler errors crash the app, just log them
        this.logger.error('Error in error handler', {
          error: handlerError instanceof Error ? handlerError.message : String(handlerError)
        })
      }
    }
  }
}
