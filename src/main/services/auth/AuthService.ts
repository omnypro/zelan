import { EventBus } from '@s/core/bus'
import { getErrorService } from '@m/services/errors'
import {
  AuthService as IAuthService,
  AuthProvider,
  AuthOptions,
  AuthResult,
  AuthStatus,
  AuthToken,
  DeviceCodeResponse
} from '@s/auth/interfaces'
import { AuthError, AuthErrorCode } from '@s/auth/errors'
import { getTwitchAuthService } from './TwitchAuthService'
import { Observable, of } from 'rxjs'

/**
 * Main authentication service that manages multiple auth providers
 */
export class AuthService implements IAuthService {
  private providerMap: Map<AuthProvider, IAuthService> = new Map()
  private initialized = false
  private eventBus: EventBus

  /**
   * Create a new AuthService
   */
  constructor(eventBus: EventBus) {
    this.eventBus = eventBus
  }

  /**
   * Initialize the authentication service and all providers
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return
    }

    try {
      // Initialize Twitch auth service
      const twitchAuthService = getTwitchAuthService(this.eventBus)
      await twitchAuthService.initialize()
      this.providerMap.set(AuthProvider.TWITCH, twitchAuthService)

      // Initialize other providers as needed

      this.initialized = true
    } catch (error) {
      getErrorService().reportError(
        error instanceof AuthError
          ? error
          : new AuthError(
              'Failed to initialize authentication service',
              AuthProvider.TWITCH, // Default provider
              AuthErrorCode.AUTHENTICATION_FAILED,
              { originalError: error },
              error instanceof Error ? error : undefined
            )
      )
      throw error
    }
  }

  /**
   * Start the authentication process for a provider
   */
  async authenticate(provider: AuthProvider, options: AuthOptions): Promise<AuthResult> {
    if (!this.initialized) {
      await this.initialize()
    }

    const authService = this.getProviderService(provider)
    return authService.authenticate(provider, options)
  }

  /**
   * Refresh the authentication token
   */
  async refreshToken(provider: AuthProvider): Promise<AuthResult> {
    if (!this.initialized) {
      await this.initialize()
    }

    const authService = this.getProviderService(provider)
    return authService.refreshToken(provider)
  }

  /**
   * Revoke the authentication token
   */
  async revokeToken(provider: AuthProvider): Promise<void> {
    if (!this.initialized) {
      await this.initialize()
    }

    const authService = this.getProviderService(provider)
    return authService.revokeToken(provider)
  }

  /**
   * Check if a provider is authenticated
   */
  isAuthenticated(provider: AuthProvider): boolean {
    try {
      const authService = this.getProviderService(provider)
      return authService.isAuthenticated(provider)
    } catch (error) {
      return false
    }
  }

  /**
   * Get the authentication token for a provider
   */
  async getToken(provider: AuthProvider): Promise<AuthToken | undefined> {
    if (!this.initialized) {
      await this.initialize()
    }

    const authService = this.getProviderService(provider)
    return authService.getToken(provider)
  }

  /**
   * Get the authentication status for a provider
   */
  getStatus(provider: AuthProvider): AuthStatus {
    try {
      const authService = this.getProviderService(provider)
      return authService.getStatus(provider)
    } catch (error) {
      throw new AuthError(
        `No authentication service available for provider ${provider}`,
        provider,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { availableProviders: Array.from(this.providerMap.keys()) }
      )
    }
  }

  /**
   * Observable of authentication status changes for a provider
   */
  status$(provider: AuthProvider): Observable<AuthStatus> {
    const authService = this.getProviderService(provider)
    return authService.status$(provider)
  }

  /**
   * Observable of authentication status changes for a provider (tRPC compatibility)
   * This method is used by the tRPC routers which use string literals
   */
  onStatusChange(providerStr: string): Observable<AuthStatus> {
    try {
      // Convert string to enum
      const provider = providerStr as AuthProvider
      const authService = this.getProviderService(provider)
      return authService.status$(provider)
    } catch (error) {
      // Return empty observable for invalid providers
      return of({
        state: 'unauthenticated',
        provider: providerStr as any,
        lastUpdated: Date.now()
      } as AuthStatus)
    }
  }

  /**
   * Observable of device code events
   * This delegates to the Twitch auth service which handles device code flow
   */
  onDeviceCode(): Observable<DeviceCodeResponse> {
    try {
      // Currently only Twitch supports device code flow
      const authService = this.getProviderService(AuthProvider.TWITCH) as any

      // Check if the service has the method
      if (authService && typeof authService.onDeviceCode === 'function') {
        return authService.onDeviceCode()
      }

      // Otherwise return empty observable
      return of({
        device_code: '',
        user_code: '',
        verification_uri: '',
        expires_in: 0,
        interval: 0
      })
    } catch (error) {
      // Return empty observable on error
      return of({
        device_code: '',
        user_code: '',
        verification_uri: '',
        expires_in: 0,
        interval: 0
      })
    }
  }

  /**
   * Get the authentication service for a provider
   */
  private getProviderService(provider: AuthProvider): IAuthService {
    const authService = this.providerMap.get(provider)
    if (!authService) {
      throw new AuthError(
        `No authentication service available for provider ${provider}`,
        provider,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { availableProviders: Array.from(this.providerMap.keys()) }
      )
    }
    return authService
  }

  /**
   * Dispose all auth service resources
   */
  dispose(): void {
    // Dispose all provider services
    for (const [, service] of this.providerMap.entries()) {
      if ('dispose' in service && typeof service.dispose === 'function') {
        service.dispose()
      }
    }
  }
}

/**
 * Singleton instance of AuthService
 */
let authServiceInstance: AuthService | null = null

/**
 * Get the authentication service instance
 */
export function getAuthService(eventBus: EventBus): AuthService {
  if (!authServiceInstance) {
    authServiceInstance = new AuthService(eventBus)
  }
  return authServiceInstance
}
