import { Observable } from 'rxjs';
import { AuthProvider, AuthOptions, AuthResult, AuthStatus, AuthToken } from './AuthTypes';

/**
 * Interface for the authentication service
 */
export interface AuthService {
  /**
   * Initialize the authentication service
   */
  initialize(): Promise<void>;

  /**
   * Start the authentication process for a provider
   * 
   * @param provider The authentication provider to use
   * @param options Authentication options for the provider
   * @returns A promise that resolves with the authentication result
   */
  authenticate(provider: AuthProvider, options: AuthOptions): Promise<AuthResult>;

  /**
   * Refresh the authentication token
   * 
   * @param provider The authentication provider
   * @returns A promise that resolves with the authentication result
   */
  refreshToken(provider: AuthProvider): Promise<AuthResult>;

  /**
   * Revoke the authentication token
   * 
   * @param provider The authentication provider
   * @returns A promise that resolves when the token is revoked
   */
  revokeToken(provider: AuthProvider): Promise<void>;

  /**
   * Check if a provider is authenticated
   * 
   * @param provider The authentication provider
   * @returns True if authenticated, false otherwise
   */
  isAuthenticated(provider: AuthProvider): boolean;

  /**
   * Get the authentication token for a provider
   * 
   * @param provider The authentication provider
   * @returns The authentication token or undefined if not authenticated
   */
  getToken(provider: AuthProvider): AuthToken | undefined;

  /**
   * Get the authentication status for a provider
   * 
   * @param provider The authentication provider
   * @returns The current authentication status
   */
  getStatus(provider: AuthProvider): AuthStatus;

  /**
   * Observable of authentication status changes for a provider
   * 
   * @param provider The authentication provider
   * @returns An observable of authentication status
   */
  status$(provider: AuthProvider): Observable<AuthStatus>;
}