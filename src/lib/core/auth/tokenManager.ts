import { z } from 'zod';
import { isRenderer } from '../../utils';

/**
 * Schema for auth tokens with validation
 */
export const TokenSchema = z.object({
  accessToken: z.string(),
  refreshToken: z.string().optional(),
  expiresAt: z.number(),
  scope: z.string().optional(),
  tokenType: z.string().default('Bearer'),
});

export type Token = z.infer<typeof TokenSchema>;

/**
 * TokenManager handles secure storage and retrieval of authentication tokens
 * using IPC to communicate with the main process's TokenStore
 */
export class TokenManager {
  private static instance: TokenManager;
  
  private constructor() {
    // No initialization needed - all operations happen through IPC
  }
  
  /**
   * Get singleton instance of TokenManager
   */
  public static getInstance(): TokenManager {
    if (!TokenManager.instance) {
      TokenManager.instance = new TokenManager();
    }
    return TokenManager.instance;
  }
  
  /**
   * Save a token for a service
   */
  public async saveToken(serviceId: string, token: Token): Promise<void> {
    try {
      // Validate the token first
      const validatedToken = TokenSchema.parse(token);
      
      // Call through IPC
      if (isRenderer()) {
        // Use tRPC from renderer
        const { client } = await import('../../trpc/client');
        await client.config.saveToken.mutate({ 
          serviceId, 
          token: validatedToken 
        });
      } else {
        // Direct call in main process
        const { TokenStore } = await import('../../../../electron/store');
        const tokenStore = TokenStore.getInstance();
        tokenStore.saveToken(serviceId, validatedToken);
      }
    } catch (error) {
      console.error('Invalid token format:', error);
      throw new Error('Failed to save token: invalid format');
    }
  }
  
  /**
   * Get a token for a service
   */
  public async getToken(serviceId: string): Promise<Token | null> {
    try {
      if (isRenderer()) {
        // Use tRPC from renderer
        const { client } = await import('../../trpc/client');
        const result = await client.config.getToken.query(serviceId);
        
        if (!result.success || !result.data) {
          return null;
        }
        
        try {
          return TokenSchema.parse(result.data);
        } catch (parseError) {
          console.error('Invalid token format:', parseError);
          return null;
        }
      } else {
        // Direct call in main process
        const { TokenStore } = await import('../../../../electron/store');
        const tokenStore = TokenStore.getInstance();
        const token = tokenStore.getToken(serviceId);
        
        if (!token) {
          return null;
        }
        
        try {
          return TokenSchema.parse(token);
        } catch (parseError) {
          console.error('Invalid token format:', parseError);
          return null;
        }
      }
    } catch (error) {
      console.error('Error retrieving token:', error);
      return null;
    }
  }
  
  /**
   * Check if a token exists and is valid
   */
  public async hasValidToken(serviceId: string): Promise<boolean> {
    try {
      if (isRenderer()) {
        // Use tRPC from renderer
        const { client } = await import('../../trpc/client');
        const result = await client.config.hasValidToken.query(serviceId);
        return result.success ? result.data.isValid : false;
      } else {
        // Direct call in main process
        const { TokenStore } = await import('../../../../electron/store');
        const tokenStore = TokenStore.getInstance();
        return tokenStore.hasValidToken(serviceId);
      }
    } catch (error) {
      console.error('Error checking token validity:', error);
      return false;
    }
  }
  
  /**
   * Delete a token
   */
  public async deleteToken(serviceId: string): Promise<void> {
    try {
      if (isRenderer()) {
        // Use tRPC from renderer
        const { client } = await import('../../trpc/client');
        const result = await client.config.deleteToken.mutate(serviceId);
        
        if (!result.success) {
          throw new Error(result.error || 'Failed to delete token');
        }
      } else {
        // Direct call in main process
        const { TokenStore } = await import('../../../../electron/store');
        const tokenStore = TokenStore.getInstance();
        tokenStore.deleteToken(serviceId);
      }
    } catch (error) {
      console.error('Error deleting token:', error);
      throw new Error(`Failed to delete token: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Clear all tokens
   */
  public async clearAllTokens(): Promise<void> {
    try {
      if (isRenderer()) {
        // Use tRPC from renderer
        const { client } = await import('../../trpc/client');
        const result = await client.config.clearAllTokens.mutate();
        
        if (!result.success) {
          throw new Error(result.error || 'Failed to clear tokens');
        }
      } else {
        // Direct call in main process
        const { TokenStore } = await import('../../../../electron/store');
        const tokenStore = TokenStore.getInstance();
        tokenStore.clearAllTokens();
      }
    } catch (error) {
      console.error('Error clearing tokens:', error);
      throw new Error(`Failed to clear tokens: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}