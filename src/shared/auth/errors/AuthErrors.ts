import { ApplicationError, ErrorCategory, ErrorSeverity } from '@s/errors/ErrorTypes';
import { AuthProvider } from '../interfaces/AuthTypes';

/**
 * Error codes specific to authentication
 */
export enum AuthErrorCode {
  TOKEN_EXPIRED = 'token_expired',
  TOKEN_INVALID = 'token_invalid',
  TOKEN_REVOKED = 'token_revoked',
  AUTHENTICATION_FAILED = 'authentication_failed',
  INVALID_GRANT = 'invalid_grant',
  INVALID_SCOPE = 'invalid_scope',
  AUTHORIZATION_PENDING = 'authorization_pending',
  SLOW_DOWN = 'slow_down',
  ACCESS_DENIED = 'access_denied',
  REQUEST_FAILED = 'request_failed',
  DEVICE_CODE_TIMEOUT = 'device_code_timeout',
  REFRESH_FAILED = 'refresh_failed',
  STORAGE_ERROR = 'storage_error',
}

/**
 * Base authentication error
 */
export class AuthError extends ApplicationError {
  provider: AuthProvider;
  
  constructor(
    message: string,
    provider: AuthProvider,
    code: AuthErrorCode,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      message,
      ErrorCategory.AUTHENTICATION,
      ErrorSeverity.WARNING,
      {
        provider,
        code,
        ...metadata
      },
      cause
    );
    
    this.provider = provider;
  }
}

/**
 * Error for expired authentication tokens
 */
export class TokenExpiredError extends AuthError {
  constructor(
    provider: AuthProvider,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      `Token for ${provider} has expired`,
      provider,
      AuthErrorCode.TOKEN_EXPIRED,
      metadata,
      cause
    );
  }
}

/**
 * Error for invalid authentication tokens
 */
export class TokenInvalidError extends AuthError {
  constructor(
    provider: AuthProvider,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      `Token for ${provider} is invalid`,
      provider,
      AuthErrorCode.TOKEN_INVALID,
      metadata,
      cause
    );
  }
}

/**
 * Error for revoked authentication tokens
 */
export class TokenRevokedError extends AuthError {
  constructor(
    provider: AuthProvider,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      `Token for ${provider} has been revoked`,
      provider,
      AuthErrorCode.TOKEN_REVOKED,
      metadata,
      cause
    );
  }
}

/**
 * Error for authentication failures
 */
export class AuthenticationFailedError extends AuthError {
  constructor(
    provider: AuthProvider,
    message: string = `Authentication failed for ${provider}`,
    code: AuthErrorCode = AuthErrorCode.AUTHENTICATION_FAILED,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      message,
      provider,
      code,
      metadata,
      cause
    );
  }
}

/**
 * Error for device code flow timeout
 */
export class DeviceCodeTimeoutError extends AuthError {
  constructor(
    provider: AuthProvider,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      `Device code authentication timed out for ${provider}`,
      provider,
      AuthErrorCode.DEVICE_CODE_TIMEOUT,
      metadata,
      cause
    );
  }
}

/**
 * Error for token refresh failures
 */
export class RefreshFailedError extends AuthError {
  constructor(
    provider: AuthProvider,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      `Failed to refresh token for ${provider}`,
      provider,
      AuthErrorCode.REFRESH_FAILED,
      metadata,
      cause
    );
  }
}

/**
 * Error for token storage issues
 */
export class StorageError extends AuthError {
  constructor(
    provider: AuthProvider,
    operation: string,
    metadata?: Record<string, unknown>,
    cause?: Error
  ) {
    super(
      `Token storage error for ${provider} during ${operation}`,
      provider,
      AuthErrorCode.STORAGE_ERROR,
      metadata,
      cause
    );
  }
}