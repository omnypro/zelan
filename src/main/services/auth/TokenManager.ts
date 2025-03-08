import crypto from 'crypto';
import { app } from 'electron';
import fs from 'fs/promises';
import path from 'path';
import { AuthProvider, AuthToken } from '@s/auth/interfaces';
import { TokenManager as ITokenManager } from '@s/auth/interfaces';
import { StorageError } from '@s/auth/errors';
import { getErrorService } from '@m/services/errors';

/**
 * Implementation of TokenManager that securely stores tokens
 * on disk with encryption
 */
export class TokenManager implements ITokenManager {
  private readonly tokenStorePath: string;
  private readonly encryptionKey: Buffer;
  private readonly algorithm = 'aes-256-gcm';
  private initialized = false;

  constructor() {
    // Use a path in the user data directory
    this.tokenStorePath = path.join(
      app.getPath('userData'),
      'auth-tokens.enc'
    );
    
    // For now, we'll use a fixed encryption key derived from the machine ID
    // In the future, we might want to use a more secure key management solution
    const machineId = app.getPath('userData'); // Using userData path as a unique identifier
    this.encryptionKey = crypto.createHash('sha256')
      .update(machineId)
      .digest();
  }

  /**
   * Initialize the token manager
   */
  async initialize(): Promise<void> {
    // Create the token store file if it doesn't exist
    try {
      await fs.access(this.tokenStorePath).catch(async () => {
        // File doesn't exist, create an empty encrypted object
        await this.writeTokens({});
      });
      
      this.initialized = true;
    } catch (error) {
      getErrorService().reportError(
        new StorageError(
          AuthProvider.TWITCH, // Using TWITCH as default
          'initialize',
          { path: this.tokenStorePath },
          error instanceof Error ? error : undefined
        )
      );
      throw error;
    }
  }

  /**
   * Save a token for a provider
   */
  async saveToken(provider: AuthProvider, token: AuthToken): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }

    try {
      // Read current tokens
      const tokens = await this.readTokens();
      
      // Update the token for the provider
      tokens[provider] = token;
      
      // Write back to disk
      await this.writeTokens(tokens);
    } catch (error) {
      getErrorService().reportError(
        new StorageError(
          provider,
          'saveToken',
          { provider },
          error instanceof Error ? error : undefined
        )
      );
      throw error;
    }
  }

  /**
   * Load a token for a provider
   */
  async loadToken(provider: AuthProvider): Promise<AuthToken | undefined> {
    if (!this.initialized) {
      await this.initialize();
    }

    try {
      const tokens = await this.readTokens();
      return tokens[provider];
    } catch (error) {
      getErrorService().reportError(
        new StorageError(
          provider,
          'loadToken',
          { provider },
          error instanceof Error ? error : undefined
        )
      );
      throw error;
    }
  }

  /**
   * Delete a token for a provider
   */
  async deleteToken(provider: AuthProvider): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }

    try {
      const tokens = await this.readTokens();
      
      // Remove the token if it exists
      if (tokens[provider]) {
        delete tokens[provider];
        await this.writeTokens(tokens);
      }
    } catch (error) {
      getErrorService().reportError(
        new StorageError(
          provider,
          'deleteToken',
          { provider },
          error instanceof Error ? error : undefined
        )
      );
      throw error;
    }
  }

  /**
   * Check if a token exists for a provider
   */
  async hasToken(provider: AuthProvider): Promise<boolean> {
    if (!this.initialized) {
      await this.initialize();
    }

    try {
      const tokens = await this.readTokens();
      return !!tokens[provider];
    } catch (error) {
      getErrorService().reportError(
        new StorageError(
          provider,
          'hasToken',
          { provider },
          error instanceof Error ? error : undefined
        )
      );
      return false;
    }
  }

  /**
   * Check if a token is expired
   */
  isTokenExpired(token: AuthToken, bufferSeconds = 300): boolean {
    if (!token.expiresAt) {
      return false;
    }

    // Check if the token expires within the buffer period
    const now = Date.now();
    const bufferMs = bufferSeconds * 1000;
    return token.expiresAt - now < bufferMs;
  }

  /**
   * Securely wipe all tokens
   */
  async clearAllTokens(): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }

    try {
      await this.writeTokens({});
    } catch (error) {
      getErrorService().reportError(
        new StorageError(
          AuthProvider.TWITCH, // Using TWITCH as default
          'clearAllTokens',
          {},
          error instanceof Error ? error : undefined
        )
      );
      throw error;
    }
  }

  /**
   * Read and decrypt tokens from disk
   */
  private async readTokens(): Promise<Record<string, AuthToken>> {
    try {
      // Read the encrypted data
      const encryptedData = await fs.readFile(this.tokenStorePath);
      
      // If the file is empty, return an empty object
      if (encryptedData.length === 0) {
        return {};
      }
      
      // Extract the IV and authentication tag
      const iv = encryptedData.subarray(0, 16);
      const authTag = encryptedData.subarray(16, 32);
      const encrypted = encryptedData.subarray(32);
      
      // Create a decipher
      const decipher = crypto.createDecipheriv(
        this.algorithm,
        this.encryptionKey,
        iv
      ) as crypto.DecipherGCM;
      
      // Set the authentication tag
      decipher.setAuthTag(authTag);
      
      // Decrypt the data
      const decrypted = Buffer.concat([
        decipher.update(encrypted),
        decipher.final()
      ]);
      
      // Parse the JSON
      return JSON.parse(decrypted.toString());
    } catch (error) {
      // If the file doesn't exist or can't be decrypted, return an empty object
      console.error('Error reading tokens:', error);
      return {};
    }
  }

  /**
   * Encrypt and write tokens to disk
   */
  private async writeTokens(tokens: Record<string, AuthToken>): Promise<void> {
    // Generate a random IV
    const iv = crypto.randomBytes(16);
    
    // Create a cipher
    const cipher = crypto.createCipheriv(
      this.algorithm,
      this.encryptionKey,
      iv
    ) as crypto.CipherGCM;
    
    // Encrypt the data
    const encrypted = Buffer.concat([
      cipher.update(Buffer.from(JSON.stringify(tokens))),
      cipher.final()
    ]);
    
    // Get the authentication tag
    const authTag = cipher.getAuthTag();
    
    // Concatenate IV, auth tag, and encrypted data
    const dataToWrite = Buffer.concat([iv, authTag, encrypted]);
    
    // Write to disk
    await fs.writeFile(this.tokenStorePath, dataToWrite);
  }
}

/**
 * Singleton instance of TokenManager
 */
let tokenManagerInstance: TokenManager | null = null;

/**
 * Get the token manager instance
 */
export function getTokenManager(): TokenManager {
  if (!tokenManagerInstance) {
    tokenManagerInstance = new TokenManager();
  }
  return tokenManagerInstance;
}