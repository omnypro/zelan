// Import TokenSchema and Token types from electron implementation for consistency
import { TokenSchema, Token } from '~/core/auth';

/**
 * TokenManager handles secure storage and retrieval of authentication tokens
 * using tRPC to communicate with the main process's TokenStore
 * 
 * Renderer process version, used by UI components
 */
export class TokenManager {
  private static instance: TokenManager;
  
  private constructor() {
    // No initialization needed - all operations happen through tRPC
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
      
      // Use tRPC from renderer
      const { client } = await import('@/lib/trpc/client');
      await client.config.saveToken.mutate({ 
        serviceId, 
        token: validatedToken 
      });
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
      // Use tRPC from renderer
      const { client } = await import('@/lib/trpc/client');
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
      // Use tRPC from renderer
      const { client } = await import('@/lib/trpc/client');
      const result = await client.config.hasValidToken.query(serviceId);
      return result.success ? result.data.isValid : false;
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
      // Use tRPC from renderer
      const { client } = await import('@/lib/trpc/client');
      const result = await client.config.deleteToken.mutate(serviceId);
      
      if (!result.success) {
        throw new Error(result.error || 'Failed to delete token');
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
      // Use tRPC from renderer
      const { client } = await import('@/lib/trpc/client');
      const result = await client.config.clearAllTokens.mutate();
      
      if (!result.success) {
        throw new Error(result.error || 'Failed to clear tokens');
      }
    } catch (error) {
      console.error('Error clearing tokens:', error);
      throw new Error(`Failed to clear tokens: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}