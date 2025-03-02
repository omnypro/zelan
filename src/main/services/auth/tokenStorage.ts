import { app } from 'electron'
import Store from 'electron-store'
import { AccessToken } from '@twurple/auth'

// Interface for stored tokens
export interface StoredToken {
  accessToken: string
  refreshToken: string
  expiresAt: number
  scope: string[]
  obtainmentTimestamp: number
}

// We'll use a simple type definition for our store
type StoreSchema = {
  tokens: Record<string, StoredToken>
}

/**
 * Service for securely storing OAuth tokens
 */
export class TokenStorage {
  private store: Store<StoreSchema>

  constructor() {
    // Create store with encryption in production
    this.store = new Store<StoreSchema>({
      name: 'auth-tokens',
      encryptionKey: app.isPackaged ? 'your-encryption-key' : undefined, // Use a secure key in production
      defaults: {
        tokens: {}
      }
    })

    // Initialize tokens object if it doesn't exist
    if (!this.store.has('tokens')) {
      this.store.set('tokens', {})
    }
  }

  /**
   * Save a token for a user
   */
  public saveToken(userId: string, token: AccessToken): void {
    const expiresAt = token.obtainmentTimestamp + (token.expiresIn || 0) * 1000

    const storedToken: StoredToken = {
      accessToken: token.accessToken,
      refreshToken: token.refreshToken || '',
      expiresAt,
      scope: token.scope || [],
      obtainmentTimestamp: token.obtainmentTimestamp
    }

    this.store.set(`tokens.${userId}`, storedToken)
  }

  /**
   * Get a token for a user
   */
  public getToken(userId: string): AccessToken | null {
    // Get all tokens
    const tokens = this.store.get('tokens') as Record<string, StoredToken>

    // Check if user token exists
    if (!tokens || !tokens[userId]) {
      return null
    }

    const storedToken = tokens[userId]

    // Convert to AccessToken format
    return {
      accessToken: storedToken.accessToken,
      refreshToken: storedToken.refreshToken,
      expiresIn: Math.floor((storedToken.expiresAt - Date.now()) / 1000),
      obtainmentTimestamp: storedToken.obtainmentTimestamp,
      scope: storedToken.scope
    }
  }

  /**
   * Remove a token for a user
   */
  public removeToken(userId: string): void {
    // Get current tokens
    const tokens = this.store.get('tokens') as Record<string, StoredToken>

    // Remove the specific user token
    if (tokens && tokens[userId]) {
      delete tokens[userId]

      // Update the store
      this.store.set('tokens', tokens)
    }
  }

  /**
   * Check if a token exists and is still valid
   */
  public hasValidToken(userId: string): boolean {
    // Get all tokens
    const tokens = this.store.get('tokens') as Record<string, StoredToken>

    // Check if user token exists
    if (!tokens || !tokens[userId]) {
      return false
    }

    const storedToken = tokens[userId]

    // Check if token is expired
    // Add a 60-second buffer to ensure we don't use tokens that are about to expire
    return storedToken.expiresAt > Date.now() + 60000
  }

  /**
   * Get all user IDs with stored tokens
   */
  public getUserIds(): string[] {
    const tokens = this.store.get('tokens') as Record<string, StoredToken>
    return Object.keys(tokens)
  }
}
