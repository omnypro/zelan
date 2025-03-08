import { EventBus } from '@s/core/bus/EventBus';
import { getErrorService } from '@m/services/errors/ErrorService';
import { 
  AuthService as IAuthService,
  AuthProvider, 
  AuthOptions, 
  AuthResult, 
  AuthStatus, 
  AuthToken 
} from '@s/auth/interfaces';
import { AuthError, AuthErrorCode } from '@s/auth/errors';
import { getTwitchAuthService } from './TwitchAuthService';
import { Observable } from 'rxjs';

/**
 * Main authentication service that manages multiple auth providers
 */
export class AuthService implements IAuthService {
  private providerMap: Map<AuthProvider, IAuthService> = new Map();
  private initialized = false;
  private eventBus: EventBus;

  /**
   * Create a new AuthService
   */
  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
  }

  /**
   * Initialize the authentication service and all providers
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      // Initialize Twitch auth service
      const twitchAuthService = getTwitchAuthService(this.eventBus);
      await twitchAuthService.initialize();
      this.providerMap.set(AuthProvider.TWITCH, twitchAuthService);

      // Initialize other providers as needed

      this.initialized = true;
    } catch (error) {
      getErrorService().reportError(
        error instanceof AuthError ? error : new AuthError(
          'Failed to initialize authentication service',
          AuthProvider.TWITCH, // Default provider
          AuthErrorCode.AUTHENTICATION_FAILED,
          { originalError: error },
          error instanceof Error ? error : undefined
        )
      );
      throw error;
    }
  }

  /**
   * Start the authentication process for a provider
   */
  async authenticate(provider: AuthProvider, options: AuthOptions): Promise<AuthResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    const authService = this.getProviderService(provider);
    return authService.authenticate(provider, options);
  }

  /**
   * Refresh the authentication token
   */
  async refreshToken(provider: AuthProvider): Promise<AuthResult> {
    if (!this.initialized) {
      await this.initialize();
    }

    const authService = this.getProviderService(provider);
    return authService.refreshToken(provider);
  }

  /**
   * Revoke the authentication token
   */
  async revokeToken(provider: AuthProvider): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }

    const authService = this.getProviderService(provider);
    return authService.revokeToken(provider);
  }

  /**
   * Check if a provider is authenticated
   */
  isAuthenticated(provider: AuthProvider): boolean {
    try {
      const authService = this.getProviderService(provider);
      return authService.isAuthenticated(provider);
    } catch (error) {
      return false;
    }
  }

  /**
   * Get the authentication token for a provider
   */
  async getToken(provider: AuthProvider): Promise<AuthToken | undefined> {
    if (!this.initialized) {
      await this.initialize();
    }

    const authService = this.getProviderService(provider);
    return authService.getToken(provider);
  }

  /**
   * Get the authentication status for a provider
   */
  getStatus(provider: AuthProvider): AuthStatus {
    try {
      const authService = this.getProviderService(provider);
      return authService.getStatus(provider);
    } catch (error) {
      throw new AuthError(
        `No authentication service available for provider ${provider}`,
        provider,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { availableProviders: Array.from(this.providerMap.keys()) }
      );
    }
  }

  /**
   * Observable of authentication status changes for a provider
   */
  status$(provider: AuthProvider): Observable<AuthStatus> {
    const authService = this.getProviderService(provider);
    return authService.status$(provider);
  }

  /**
   * Get the authentication service for a provider
   */
  private getProviderService(provider: AuthProvider): IAuthService {
    const authService = this.providerMap.get(provider);
    if (!authService) {
      throw new AuthError(
        `No authentication service available for provider ${provider}`,
        provider,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { availableProviders: Array.from(this.providerMap.keys()) }
      );
    }
    return authService;
  }

  /**
   * Dispose all auth service resources
   */
  dispose(): void {
    // Dispose all provider services
    for (const [provider, service] of this.providerMap.entries()) {
      if ('dispose' in service && typeof service.dispose === 'function') {
        service.dispose();
      }
    }
  }
}

/**
 * Singleton instance of AuthService
 */
let authServiceInstance: AuthService | null = null;

/**
 * Get the authentication service instance
 */
export function getAuthService(eventBus: EventBus): AuthService {
  if (!authServiceInstance) {
    authServiceInstance = new AuthService(eventBus);
  }
  return authServiceInstance;
}