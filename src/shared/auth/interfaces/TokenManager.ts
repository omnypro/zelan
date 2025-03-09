import { AuthProvider, AuthToken } from './AuthTypes'

/**
 * Interface for token management operations
 */
export interface TokenManager {
  /**
   * Initialize the token manager
   */
  initialize(): Promise<void>

  /**
   * Save a token for a provider
   *
   * @param provider The authentication provider
   * @param token The token to save
   * @returns A promise that resolves when the token is saved
   */
  saveToken(provider: AuthProvider, token: AuthToken): Promise<void>

  /**
   * Load a token for a provider
   *
   * @param provider The authentication provider
   * @returns The token or undefined if not found
   */
  loadToken(provider: AuthProvider): Promise<AuthToken | undefined>

  /**
   * Delete a token for a provider
   *
   * @param provider The authentication provider
   * @returns A promise that resolves when the token is deleted
   */
  deleteToken(provider: AuthProvider): Promise<void>

  /**
   * Check if a token exists for a provider
   *
   * @param provider The authentication provider
   * @returns True if a token exists, false otherwise
   */
  hasToken(provider: AuthProvider): Promise<boolean>

  /**
   * Check if a token is expired
   *
   * @param token The token to check
   * @param bufferSeconds Optional buffer time in seconds before expiration
   * @returns True if the token is expired, false otherwise
   */
  isTokenExpired(token: AuthToken, bufferSeconds?: number): boolean

  /**
   * Securely wipe all tokens
   *
   * @returns A promise that resolves when all tokens are deleted
   */
  clearAllTokens(): Promise<void>
}
