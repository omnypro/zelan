import { BehaviorSubject, Observable, timer, Subscription } from 'rxjs';
import { TokenManager, Token } from './tokenManager';
import { EventBus, EventType, createEvent, BaseEventSchema } from '../events';
import { z } from 'zod';

/**
 * Authentication state enum
 */
export enum AuthState {
  UNKNOWN = 'unknown',
  UNAUTHENTICATED = 'unauthenticated',
  AUTHENTICATING = 'authenticating',
  AUTHENTICATED = 'authenticated',
  REFRESHING = 'refreshing',
  ERROR = 'error',
}

/**
 * Auth provider interface that all service-specific auth providers must implement
 */
export interface AuthProvider {
  readonly serviceId: string;
  startAuth(): Promise<void>;
  refreshToken(refreshToken: string): Promise<Token>;
  revokeToken(token: Token): Promise<void>;
}

/**
 * Auth event schema built on top of the base event
 */
export const AuthEventSchema = BaseEventSchema.extend({
  serviceId: z.string(),
  state: z.nativeEnum(AuthState),
  error: z.string().optional(),
});

export type AuthEvent = z.infer<typeof AuthEventSchema>;

/**
 * AuthService manages the authentication state and lifecycle
 */
export class AuthService {
  private static instance: AuthService;
  private eventBus: EventBus = EventBus.getInstance();
  private tokenManager: TokenManager = TokenManager.getInstance();
  private providers: Map<string, AuthProvider> = new Map();
  private refreshTimers: Map<string, Subscription> = new Map();
  private authStates: Map<string, BehaviorSubject<AuthState>> = new Map();
  
  private constructor() {
    // Initialize
  }
  
  /**
   * Get singleton instance of AuthService
   */
  public static getInstance(): AuthService {
    if (!AuthService.instance) {
      AuthService.instance = new AuthService();
    }
    return AuthService.instance;
  }
  
  /**
   * Register an auth provider
   */
  public async registerProvider(provider: AuthProvider): Promise<void> {
    if (this.providers.has(provider.serviceId)) {
      throw new Error(`Provider for ${provider.serviceId} already registered`);
    }
    
    this.providers.set(provider.serviceId, provider);
    
    // Check token validity and set initial state
    const isValid = await this.tokenManager.hasValidToken(provider.serviceId);
    this.authStates.set(
      provider.serviceId, 
      new BehaviorSubject<AuthState>(
        isValid ? AuthState.AUTHENTICATED : AuthState.UNAUTHENTICATED
      )
    );
    
    // Set up refresh timer if we have a valid token
    const token = await this.tokenManager.getToken(provider.serviceId);
    if (token && token.refreshToken) {
      this.scheduleTokenRefresh(provider.serviceId, token);
    }
  }
  
  /**
   * Start authentication for a specific service
   */
  public async authenticate(serviceId: string): Promise<void> {
    const provider = this.getProvider(serviceId);
    
    try {
      // Update state to authenticating
      this.updateAuthState(serviceId, AuthState.AUTHENTICATING);
      
      // Publish auth started event
      this.eventBus.publish(createEvent(
        AuthEventSchema,
        {
          type: EventType.AUTH_STARTED,
          source: 'auth-service',
          serviceId,
          state: AuthState.AUTHENTICATING,
        }
      ));
      
      // Start the authentication process
      await provider.startAuth();
      
      // Auth completed events will be published by the provider when complete
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // Update state to error
      this.updateAuthState(serviceId, AuthState.ERROR);
      
      // Publish auth failed event
      this.eventBus.publish(createEvent(
        AuthEventSchema,
        {
          type: EventType.AUTH_FAILED,
          source: 'auth-service',
          serviceId,
          state: AuthState.ERROR,
          error: errorMessage,
        }
      ));
      
      throw error;
    }
  }
  
  /**
   * Completes the authentication process with a token
   * This is typically called by the auth provider
   */
  public async completeAuthentication(serviceId: string, token: Token): Promise<void> {
    try {
      // Save the token
      await this.tokenManager.saveToken(serviceId, token);
      
      // Update state to authenticated
      this.updateAuthState(serviceId, AuthState.AUTHENTICATED);
      
      // Publish auth completed event
      this.eventBus.publish(createEvent(
        AuthEventSchema,
        {
          type: EventType.AUTH_COMPLETED,
          source: 'auth-service',
          serviceId,
          state: AuthState.AUTHENTICATED,
        }
      ));
      
      // Schedule token refresh if we have a refresh token
      if (token.refreshToken) {
        this.scheduleTokenRefresh(serviceId, token);
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // Update state to error
      this.updateAuthState(serviceId, AuthState.ERROR);
      
      // Publish auth failed event
      this.eventBus.publish(createEvent(
        AuthEventSchema,
        {
          type: EventType.AUTH_FAILED,
          source: 'auth-service',
          serviceId,
          state: AuthState.ERROR,
          error: errorMessage,
        }
      ));
    }
  }
  
  /**
   * Get the current auth state for a service
   */
  public getAuthState(serviceId: string): AuthState {
    const state$ = this.authStates.get(serviceId);
    return state$ ? state$.getValue() : AuthState.UNKNOWN;
  }
  
  /**
   * Get the auth state as an observable for a service
   */
  public authState$(serviceId: string): Observable<AuthState> {
    if (!this.authStates.has(serviceId)) {
      this.authStates.set(serviceId, new BehaviorSubject<AuthState>(AuthState.UNKNOWN));
    }
    return this.authStates.get(serviceId)!.asObservable();
  }
  
  /**
   * Get the token for a service
   */
  public async getToken(serviceId: string): Promise<Token | null> {
    return await this.tokenManager.getToken(serviceId);
  }
  
  /**
   * Logout from a service
   */
  public async logout(serviceId: string): Promise<void> {
    try {
      const token = await this.tokenManager.getToken(serviceId);
      const provider = this.getProvider(serviceId);
      
      // Cancel any scheduled refresh
      this.cancelTokenRefresh(serviceId);
      
      // Revoke token if we have one
      if (token) {
        await provider.revokeToken(token);
      }
      
      // Delete the token
      await this.tokenManager.deleteToken(serviceId);
      
      // Update state to unauthenticated
      this.updateAuthState(serviceId, AuthState.UNAUTHENTICATED);
    } catch (error) {
      console.error(`Error logging out of ${serviceId}:`, error);
      
      // Still delete the token and update state
      await this.tokenManager.deleteToken(serviceId);
      this.updateAuthState(serviceId, AuthState.UNAUTHENTICATED);
    }
  }
  
  /**
   * Get a provider by service ID
   */
  private getProvider(serviceId: string): AuthProvider {
    const provider = this.providers.get(serviceId);
    if (!provider) {
      throw new Error(`No provider registered for ${serviceId}`);
    }
    return provider;
  }
  
  /**
   * Update the auth state for a service
   */
  private updateAuthState(serviceId: string, state: AuthState): void {
    if (!this.authStates.has(serviceId)) {
      this.authStates.set(serviceId, new BehaviorSubject<AuthState>(state));
    } else {
      this.authStates.get(serviceId)!.next(state);
    }
  }
  
  /**
   * Schedule a token refresh
   */
  private scheduleTokenRefresh(serviceId: string, token: Token): void {
    // Cancel any existing refresh timer
    this.cancelTokenRefresh(serviceId);
    
    // Calculate time until refresh (30 seconds before expiry)
    const now = Date.now();
    const refreshTime = Math.max(0, token.expiresAt - now - 30000);
    
    // Schedule refresh
    this.refreshTimers.set(
      serviceId,
      timer(refreshTime).subscribe(() => this.refreshToken(serviceId))
    );
  }
  
  /**
   * Cancel a scheduled token refresh
   */
  private cancelTokenRefresh(serviceId: string): void {
    const subscription = this.refreshTimers.get(serviceId);
    if (subscription) {
      subscription.unsubscribe();
      this.refreshTimers.delete(serviceId);
    }
  }
  
  /**
   * Refresh a token
   */
  private async refreshToken(serviceId: string): Promise<void> {
    const provider = this.getProvider(serviceId);
    const token = await this.tokenManager.getToken(serviceId);
    
    if (!token || !token.refreshToken) {
      // No token or refresh token
      this.updateAuthState(serviceId, AuthState.UNAUTHENTICATED);
      return;
    }
    
    try {
      // Update state to refreshing
      this.updateAuthState(serviceId, AuthState.REFRESHING);
      
      // Refresh the token
      const newToken = await provider.refreshToken(token.refreshToken);
      
      // Save the new token
      await this.tokenManager.saveToken(serviceId, newToken);
      
      // Update state to authenticated
      this.updateAuthState(serviceId, AuthState.AUTHENTICATED);
      
      // Publish token refreshed event
      this.eventBus.publish(createEvent(
        AuthEventSchema,
        {
          type: EventType.AUTH_TOKEN_REFRESHED,
          source: 'auth-service',
          serviceId,
          state: AuthState.AUTHENTICATED,
        }
      ));
      
      // Schedule the next refresh
      this.scheduleTokenRefresh(serviceId, newToken);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // Update state to error, then unauthenticated
      this.updateAuthState(serviceId, AuthState.ERROR);
      // Use a Promise with await instead of setTimeout for consistent async pattern
      await new Promise(resolve => setTimeout(resolve, 1000));
      this.updateAuthState(serviceId, AuthState.UNAUTHENTICATED);
      
      // Publish token expired event
      this.eventBus.publish(createEvent(
        AuthEventSchema,
        {
          type: EventType.AUTH_TOKEN_EXPIRED,
          source: 'auth-service',
          serviceId,
          state: AuthState.ERROR,
          error: errorMessage,
        }
      ));
      
      // Delete the invalid token
      await this.tokenManager.deleteToken(serviceId);
    }
  }
}