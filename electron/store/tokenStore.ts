import Store from 'electron-store';
import { z } from 'zod';

/**
 * Schema for auth tokens with validation
 */
export const TokenSchema = z.object({
  accessToken: z.string(),
  refreshToken: z.string().optional(),
  expiresAt: z.number(),
  scopes: z.array(z.string()).default([]),
  tokenType: z.string().default('bearer'),
});

export type Token = z.infer<typeof TokenSchema>;

/**
 * Schema for token store
 */
export const TokenStoreSchema = z.record(z.string(), TokenSchema);

/**
 * Manager for authentication tokens
 * Handles secure storage and retrieval of authentication tokens
 */
export class TokenStore {
  private static instance: TokenStore;
  private store: Store<z.infer<typeof TokenStoreSchema>>;
  
  private constructor() {
    this.store = new Store({
      name: 'auth-tokens',
      encryptionKey: 'app-specific-encryption-key', // In production, use a secure key
    });
  }
  
  /**
   * Get singleton instance of TokenStore
   */
  public static getInstance(): TokenStore {
    if (!TokenStore.instance) {
      TokenStore.instance = new TokenStore();
    }
    return TokenStore.instance;
  }
  
  /**
   * Save a token for a service
   */
  public saveToken(serviceId: string, token: Token): void {
    try {
      const validatedToken = TokenSchema.parse(token);
      this.store.set(serviceId, validatedToken);
    } catch (error) {
      console.error('Invalid token format:', error);
      throw new Error('Failed to save token: invalid format');
    }
  }
  
  /**
   * Get a token for a service
   */
  public getToken(serviceId: string): Token | null {
    try {
      const token = this.store.get(serviceId);
      return token ? TokenSchema.parse(token) : null;
    } catch (error) {
      console.error('Error retrieving token:', error);
      return null;
    }
  }
  
  /**
   * Check if a token exists and is valid
   */
  public hasValidToken(serviceId: string): boolean {
    const token = this.getToken(serviceId);
    if (!token) return false;
    
    // Check if token is expired (with 60s buffer)
    return token.expiresAt > Date.now() + 60000;
  }
  
  /**
   * Delete a token
   */
  public deleteToken(serviceId: string): void {
    this.store.delete(serviceId);
  }
  
  /**
   * Clear all tokens
   */
  public clearAllTokens(): void {
    this.store.clear();
  }
}