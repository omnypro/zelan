/**
 * Standardized error categories for application errors
 */
export enum ErrorCategory {
  SYSTEM = 'system',
  NETWORK = 'network',
  ADAPTER = 'adapter',
  CONFIG = 'config',
  VALIDATION = 'validation',
  AUTHENTICATION = 'authentication',
  RUNTIME = 'runtime',
  IPC = 'ipc',
  USER = 'user'
}

/**
 * Error severity levels
 */
export enum ErrorSeverity {
  DEBUG = 'debug',
  INFO = 'info',
  WARNING = 'warning',
  ERROR = 'error',
  CRITICAL = 'critical'
}

/**
 * Error metadata for additional context
 */
export interface ErrorMetadata {
  source?: string;
  component?: string;
  operation?: string;
  details?: Record<string, unknown>;
  recoverable?: boolean;
  [key: string]: unknown;
}

/**
 * Extended error with additional context
 */
export class ApplicationError extends Error {
  readonly category: ErrorCategory;
  readonly severity: ErrorSeverity;
  readonly metadata: ErrorMetadata;
  readonly timestamp: number;
  readonly originalError?: Error;

  constructor(
    message: string,
    category: ErrorCategory = ErrorCategory.RUNTIME,
    severity: ErrorSeverity = ErrorSeverity.ERROR,
    metadata: ErrorMetadata = {},
    originalError?: Error
  ) {
    super(message);
    this.name = 'ApplicationError';
    this.category = category;
    this.severity = severity;
    this.metadata = metadata;
    this.timestamp = Date.now();
    this.originalError = originalError;

    // Capture stack trace
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, ApplicationError);
    }

    // If this wraps another error, preserve its stack
    if (originalError?.stack) {
      this.stack = `${this.stack}\nCaused by: ${originalError.stack}`;
    }
  }

  /**
   * Convert to a plain object suitable for serialization
   */
  toJSON(): Record<string, unknown> {
    return {
      name: this.name,
      message: this.message,
      category: this.category,
      severity: this.severity,
      metadata: this.metadata,
      timestamp: this.timestamp,
      stack: this.stack,
      cause: this.originalError ? this.originalError.message : undefined
    };
  }

  /**
   * Create a system error
   */
  static system(message: string, severity: ErrorSeverity = ErrorSeverity.ERROR, metadata?: ErrorMetadata, originalError?: Error): ApplicationError {
    return new ApplicationError(message, ErrorCategory.SYSTEM, severity, metadata, originalError);
  }

  /**
   * Create a network error
   */
  static network(message: string, severity: ErrorSeverity = ErrorSeverity.ERROR, metadata?: ErrorMetadata, originalError?: Error): ApplicationError {
    return new ApplicationError(message, ErrorCategory.NETWORK, severity, metadata, originalError);
  }

  /**
   * Create an adapter error
   */
  static adapter(message: string, severity: ErrorSeverity = ErrorSeverity.ERROR, metadata?: ErrorMetadata, originalError?: Error): ApplicationError {
    return new ApplicationError(message, ErrorCategory.ADAPTER, severity, metadata, originalError);
  }

  /**
   * Create a validation error
   */
  static validation(message: string, severity: ErrorSeverity = ErrorSeverity.WARNING, metadata?: ErrorMetadata, originalError?: Error): ApplicationError {
    return new ApplicationError(message, ErrorCategory.VALIDATION, severity, metadata, originalError);
  }

  /**
   * Create a configuration error
   */
  static config(message: string, severity: ErrorSeverity = ErrorSeverity.ERROR, metadata?: ErrorMetadata, originalError?: Error): ApplicationError {
    return new ApplicationError(message, ErrorCategory.CONFIG, severity, metadata, originalError);
  }
}