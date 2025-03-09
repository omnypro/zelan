import { BehaviorSubject, Observable } from 'rxjs'
import { EventBus } from '@s/core/bus'
import { EventCategory } from '@s/types/events'
import { createEvent } from '@s/core/events'
import { getErrorService } from '@m/services/errors'
import { getTokenManager } from './TokenManager'
import { TokenManager } from '@s/auth/interfaces/TokenManager'
import { getLoggingService, ComponentLogger } from '@m/services/logging'
import {
  AuthService,
  AuthProvider,
  AuthOptions,
  AuthResult,
  AuthStatus,
  AuthState,
  AuthToken
} from '@s/auth/interfaces'
import { AuthError, AuthErrorCode, TokenExpiredError, RefreshFailedError } from '@s/auth/errors'
import { SubscriptionManager } from '@s/utils/subscription-manager'

/**
 * Base implementation of the AuthService interface
 */
export abstract class BaseAuthService implements AuthService {
  protected statusMap: Map<AuthProvider, BehaviorSubject<AuthStatus>> = new Map()
  protected tokenManager: TokenManager
  protected eventBus: EventBus
  protected subscriptionManager = new SubscriptionManager()
  protected initialized = false
  protected logger: ComponentLogger

  /**
   * Create a new BaseAuthService
   */
  constructor(eventBus: EventBus) {
    this.eventBus = eventBus
    this.tokenManager = getTokenManager()
    this.logger = getLoggingService().createLogger('BaseAuthService')
  }

  /**
   * Initialize the authentication service
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return
    }

    try {
      // Initialize the token manager
      await this.tokenManager.initialize()

      // Set up status for each provider
      for (const provider of Object.values(AuthProvider)) {
        // Create a status subject if it doesn't exist
        if (!this.statusMap.has(provider)) {
          const initialStatus: AuthStatus = {
            state: AuthState.UNAUTHENTICATED,
            provider,
            lastUpdated: Date.now()
          }
          this.statusMap.set(provider, new BehaviorSubject(initialStatus))
        }

        // Check if we have a token for this provider
        const token = await this.tokenManager.loadToken(provider)
        if (token) {
          // Check if the token is expired
          if (this.tokenManager.isTokenExpired(token)) {
            // Try to refresh the token
            await this.refreshToken(provider).catch((error) => {
              this.logger.error(`Failed to refresh token for ${provider}`, {
                error: error instanceof Error ? error.message : String(error)
              })
              // Update status to error
              this.updateStatus(provider, {
                state: AuthState.ERROR,
                error: error instanceof Error ? error : new Error(String(error))
              })
            })
          } else {
            // Token is valid, update status with user data from metadata
            this.updateStatus(provider, {
              state: AuthState.AUTHENTICATED,
              expiresAt: token.expiresAt,
              userId: token.metadata?.userId,
              username: token.metadata?.username
            })
          }
        }
      }

      // Set up token refresh timers
      this.setupTokenRefreshTimers()

      this.initialized = true
    } catch (error) {
      getErrorService().reportError(
        error instanceof AuthError
          ? error
          : new AuthError(
              'Failed to initialize authentication service',
              AuthProvider.TWITCH, // Default provider
              AuthErrorCode.AUTHENTICATION_FAILED,
              {},
              error instanceof Error ? error : undefined
            )
      )
      throw error
    }
  }

  /**
   * Abstract method to authenticate with a provider
   */
  abstract authenticate(provider: AuthProvider, options: AuthOptions): Promise<AuthResult>

  /**
   * Refresh the authentication token
   */
  async refreshToken(provider: AuthProvider): Promise<AuthResult> {
    if (!this.initialized) {
      await this.initialize()
    }

    try {
      // Get the current token
      const token = await this.tokenManager.loadToken(provider)
      if (!token || !token.refreshToken) {
        throw new RefreshFailedError(provider, { reason: 'No refresh token available' })
      }

      // Update status to authenticating
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATING
      })

      // Provider-specific token refresh
      const result = await this.refreshTokenImplementation(provider, token)

      if (!result.success || !result.token) {
        throw new RefreshFailedError(
          provider,
          {
            reason: result.error ? result.error.message : 'Unknown error',
            originalError: result.error
          },
          result.error
        )
      }

      // Save the new token
      await this.tokenManager.saveToken(provider, result.token)

      // Update status
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATED,
        expiresAt: result.token.expiresAt,
        userId: result.userId,
        username: result.username
      })

      // Publish auth event
      this.publishAuthEvent(provider, 'token_refreshed')

      return result
    } catch (error) {
      // Update status to error
      this.updateStatus(provider, {
        state: AuthState.ERROR,
        error: error instanceof Error ? error : new Error(String(error))
      })

      // Report the error
      getErrorService().reportError(
        error instanceof AuthError
          ? error
          : new RefreshFailedError(provider, { originalError: error }, error instanceof Error ? error : undefined)
      )

      // Publish auth error event
      this.publishAuthEvent(provider, 'refresh_failed', {
        error: error instanceof Error ? error.message : String(error)
      })

      throw error
    }
  }

  /**
   * Revoke the authentication token
   */
  async revokeToken(provider: AuthProvider): Promise<void> {
    if (!this.initialized) {
      await this.initialize()
    }

    try {
      // Get the current token
      const token = await this.tokenManager.loadToken(provider)
      if (!token) {
        // No token to revoke
        return
      }

      // Update status to authenticating
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATING
      })

      // Provider-specific token revocation
      await this.revokeTokenImplementation(provider, token)

      // Delete the token
      await this.tokenManager.deleteToken(provider)

      // Update status
      this.updateStatus(provider, {
        state: AuthState.UNAUTHENTICATED
      })

      // Publish auth event
      this.publishAuthEvent(provider, 'token_revoked')
    } catch (error) {
      // Update status to error
      this.updateStatus(provider, {
        state: AuthState.ERROR,
        error: error instanceof Error ? error : new Error(String(error))
      })

      // Report the error
      getErrorService().reportError(
        error instanceof AuthError
          ? error
          : new AuthError(
              `Failed to revoke token for ${provider}`,
              provider,
              AuthErrorCode.TOKEN_REVOKED,
              {},
              error instanceof Error ? error : undefined
            )
      )

      // Publish auth error event
      this.publishAuthEvent(provider, 'revocation_failed', {
        error: error instanceof Error ? error.message : String(error)
      })

      throw error
    }
  }

  /**
   * Check if a provider is authenticated
   */
  isAuthenticated(provider: AuthProvider): boolean {
    const status = this.getStatus(provider)
    return status.state === AuthState.AUTHENTICATED
  }

  /**
   * Get the authentication token for a provider
   */
  async getToken(provider: AuthProvider): Promise<AuthToken | undefined> {
    if (!this.initialized) {
      await this.initialize()
    }

    const token = await this.tokenManager.loadToken(provider)
    if (!token) {
      return undefined
    }

    // Check if the token is expired
    if (this.tokenManager.isTokenExpired(token)) {
      try {
        // Try to refresh the token
        const result = await this.refreshToken(provider)
        return result.token
      } catch (error) {
        // Token refresh failed
        this.logger.error(`Failed to refresh token for ${provider}`, {
          error: error instanceof Error ? error.message : String(error)
        })
        throw new TokenExpiredError(provider)
      }
    }

    return token
  }

  /**
   * Get the authentication status for a provider
   */
  getStatus(provider: AuthProvider): AuthStatus {
    // Get or create the status subject
    const statusSubject = this.getStatusSubject(provider)
    return statusSubject.getValue()
  }

  /**
   * Observable of authentication status changes for a provider
   */
  status$(provider: AuthProvider): Observable<AuthStatus> {
    // Get or create the status subject
    const statusSubject = this.getStatusSubject(provider)
    return statusSubject.asObservable()
  }

  /**
   * Create or get a status subject for a provider
   */
  protected getStatusSubject(provider: AuthProvider): BehaviorSubject<AuthStatus> {
    // Check if we already have a subject
    let subject = this.statusMap.get(provider)

    // Create a new subject if it doesn't exist
    if (!subject) {
      const initialStatus: AuthStatus = {
        state: AuthState.UNAUTHENTICATED,
        provider,
        lastUpdated: Date.now()
      }

      subject = new BehaviorSubject<AuthStatus>(initialStatus)
      this.statusMap.set(provider, subject)
    }

    return subject
  }

  /**
   * Update the authentication status for a provider
   */
  protected updateStatus(provider: AuthProvider, update: Partial<Omit<AuthStatus, 'provider' | 'lastUpdated'>>): void {
    // Get the current status
    const statusSubject = this.getStatusSubject(provider)
    const currentStatus = statusSubject.getValue()

    // Create the new status
    const newStatus: AuthStatus = {
      ...currentStatus,
      ...update,
      provider,
      lastUpdated: Date.now()
    }

    // Update the status
    statusSubject.next(newStatus)

    // Publish auth status event
    this.publishAuthEvent(provider, 'status_changed', {
      status: newStatus.state,
      error: newStatus.error ? newStatus.error.message : undefined
    })
  }

  /**
   * Publish an authentication event
   */
  protected publishAuthEvent(provider: AuthProvider, type: string, payload: Record<string, unknown> = {}): void {
    this.eventBus.publish(
      createEvent(
        EventCategory.SERVICE, // Using SERVICE category for auth events
        type,
        {
          provider,
          timestamp: Date.now(),
          ...payload
        },
        provider
      )
    )
  }

  // Store for our token refresh timers
  private tokenRefreshTimers: Map<string, NodeJS.Timeout> = new Map()

  /**
   * Set up timers to refresh tokens before they expire
   */
  protected setupTokenRefreshTimers(): void {
    // Clear existing timers
    this.clearTokenRefreshTimers()

    // Set up refresh timers for all providers
    for (const provider of Object.values(AuthProvider)) {
      // Get status observable for this provider
      const status$ = this.status$(provider)

      // Subscribe to status changes
      const subscription = status$.subscribe(async (status) => {
        // Only set up refresh timer for authenticated status with an expiration
        if (status.state === AuthState.AUTHENTICATED && status.expiresAt) {
          const now = Date.now()
          const expiresIn = status.expiresAt - now

          // Buffer time (5 minutes before expiration)
          const bufferTime = 5 * 60 * 1000

          // Calculate refresh time (expiration - buffer)
          const refreshTime = expiresIn - bufferTime

          // Only set up a timer if refresh time is positive
          if (refreshTime > 0) {
            // Clear any existing timer for this provider
            this.clearTokenRefreshTimer(provider)

            // Set up a timer to refresh the token
            const timerId = setTimeout(async () => {
              try {
                await this.refreshToken(provider)
              } catch (error) {
                this.logger.error(`Failed to refresh token for ${provider}`, {
                  error: error instanceof Error ? error.message : String(error)
                })
              }
              // Remove the timer ID after it's been executed
              this.tokenRefreshTimers.delete(provider)
            }, refreshTime)

            // Store the timer ID for cleanup
            this.tokenRefreshTimers.set(provider, timerId)
          }
        }
      })

      // Add the subscription to our manager
      this.subscriptionManager.add(subscription)
    }
  }

  /**
   * Clear a specific token refresh timer
   */
  private clearTokenRefreshTimer(provider: AuthProvider): void {
    const timerId = this.tokenRefreshTimers.get(provider)
    if (timerId) {
      clearTimeout(timerId)
      this.tokenRefreshTimers.delete(provider)
    }
  }

  /**
   * Clear all token refresh timers
   */
  private clearTokenRefreshTimers(): void {
    this.tokenRefreshTimers.forEach((timerId) => {
      clearTimeout(timerId)
    })
    this.tokenRefreshTimers.clear()
  }

  /**
   * Abstract method to refresh a token
   */
  protected abstract refreshTokenImplementation(provider: AuthProvider, token: AuthToken): Promise<AuthResult>

  /**
   * Abstract method to revoke a token
   */
  protected abstract revokeTokenImplementation(provider: AuthProvider, token: AuthToken): Promise<void>

  /**
   * Cleanup resources
   */
  public dispose(): void {
    // Clear all subscriptions
    this.subscriptionManager.unsubscribeAll()

    // Clear token refresh timers
    this.clearTokenRefreshTimers()
  }
}
