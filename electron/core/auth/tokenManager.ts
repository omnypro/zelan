import { TokenStore, TokenSchema, Token } from '../../store/tokenStore';

/**
 * TokenManager handles secure storage and retrieval of authentication tokens
 * Main process only
 */
export class TokenManager {
  private static instance: TokenManager;
  
  private constructor() {
    // No initialization needed - all operations happen through TokenStore
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
      
      // Direct call to token store
      const tokenStore = TokenStore.getInstance();
      tokenStore.saveToken(serviceId, validatedToken);
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
      // Direct call to token store
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
      // Direct call to token store
      const tokenStore = TokenStore.getInstance();
      return tokenStore.hasValidToken(serviceId);
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
      // Direct call to token store
      const tokenStore = TokenStore.getInstance();
      tokenStore.deleteToken(serviceId);
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
      // Direct call to token store
      const tokenStore = TokenStore.getInstance();
      tokenStore.clearAllTokens();
    } catch (error) {
      console.error('Error clearing tokens:', error);
      throw new Error(`Failed to clear tokens: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}