import { EventBus } from '@s/core/bus/EventBus';
import { BaseAuthService } from './BaseAuthService';
import { getErrorService } from '@m/services/errors/ErrorService';
import { 
  AuthProvider, 
  AuthOptions, 
  AuthResult, 
  AuthToken, 
  DeviceCodeResponse,
  AuthState 
} from '@s/auth/interfaces';
import { 
  AuthError, 
  AuthErrorCode,
  AuthenticationFailedError, 
  DeviceCodeTimeoutError, 
  RefreshFailedError 
} from '@s/auth/errors';

import { 
  AccessToken, 
  RefreshingAuthProvider as TwitchAuthProvider,
  exchangeDeviceCode,
  revokeToken,
  getDeviceCode
} from '@twurple/auth';

/**
 * Handles authentication with the Twitch API
 */
export class TwitchAuthService extends BaseAuthService {
  /**
   * Default scopes for Twitch authentication
   */
  private readonly DEFAULT_SCOPES = [
    'user:read:email',
    'channel:read:subscriptions',
    'channel:read:redemptions',
    'channel:read:polls',
    'channel:read:predictions',
    'chat:read'
  ];

  /**
   * Create a new TwitchAuthService
   */
  constructor(eventBus: EventBus) {
    super(eventBus);
  }

  /**
   * Authenticate with Twitch using device code flow
   */
  async authenticate(
    provider: AuthProvider,
    options: AuthOptions
  ): Promise<AuthResult> {
    if (provider !== AuthProvider.TWITCH) {
      throw new AuthenticationFailedError(
        provider,
        `Provider ${provider} is not supported by TwitchAuthService`,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { supportedProvider: AuthProvider.TWITCH }
      );
    }

    if (!this.initialized) {
      await this.initialize();
    }

    try {
      // Update status to authenticating
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATING
      });

      // Get device code
      const deviceCode = await this.getDeviceCode(options);

      // Publish device code event
      this.publishAuthEvent(provider, 'device_code_received', {
        user_code: deviceCode.user_code,
        verification_uri: deviceCode.verification_uri,
        verification_uri_complete: deviceCode.verification_uri_complete,
        expires_in: deviceCode.expires_in
      });

      // Exchange device code for token
      const token = await this.pollForAccessToken(deviceCode, options);

      // Get user information
      const userData = await this.getUserInfo(token);

      // Convert token format
      const authToken: AuthToken = {
        accessToken: token.accessToken,
        refreshToken: token.refreshToken,
        expiresAt: Date.now() + token.expiresIn * 1000,
        scope: token.scope,
        tokenType: 'bearer'
      };

      // Save the token
      await this.tokenManager.saveToken(provider, authToken);

      // Update status
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATED,
        expiresAt: authToken.expiresAt,
        userId: userData.id,
        username: userData.login
      });

      // Publish auth event
      this.publishAuthEvent(provider, 'authenticated', {
        userId: userData.id,
        username: userData.login
      });

      return {
        success: true,
        token: authToken,
        userId: userData.id,
        username: userData.login
      };
    } catch (error) {
      // Update status to error
      this.updateStatus(provider, {
        state: AuthState.ERROR,
        error: error instanceof Error ? error : new Error(String(error))
      });

      // Report the error
      getErrorService().reportError(
        error instanceof AuthError ? error : new AuthenticationFailedError(
          provider,
          String(error),
          AuthErrorCode.AUTHENTICATION_FAILED,
          { originalError: error },
          error instanceof Error ? error : undefined
        )
      );

      // Publish auth error event
      this.publishAuthEvent(provider, 'authentication_failed', {
        error: error instanceof Error ? error.message : String(error)
      });

      return {
        success: false,
        error: error instanceof Error ? error : new Error(String(error))
      };
    }
  }

  /**
   * Get a device code for authentication
   */
  private async getDeviceCode(options: AuthOptions): Promise<DeviceCodeResponse> {
    try {
      const { clientId, scopes = this.DEFAULT_SCOPES } = options;

      // Get device code from Twitch
      const deviceCodeData = await getDeviceCode(clientId, scopes);

      return {
        device_code: deviceCodeData.deviceCode,
        user_code: deviceCodeData.userCode,
        verification_uri: deviceCodeData.verificationUri,
        verification_uri_complete: deviceCodeData.verificationUriComplete,
        expires_in: deviceCodeData.expiresIn,
        interval: deviceCodeData.interval
      };
    } catch (error) {
      console.error('Error getting device code:', error);
      throw new AuthenticationFailedError(
        AuthProvider.TWITCH,
        `Failed to get device code: ${error instanceof Error ? error.message : String(error)}`,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { originalError: error },
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Poll for access token using device code
   */
  private async pollForAccessToken(
    deviceCode: DeviceCodeResponse,
    options: AuthOptions
  ): Promise<AccessToken> {
    const { clientId } = options;
    const startTime = Date.now();
    const timeout = deviceCode.expires_in * 1000;

    // Initial polling delay
    let pollInterval = (deviceCode.interval || 5) * 1000;

    while (true) {
      try {
        // Check if we've timed out
        if (Date.now() - startTime > timeout) {
          throw new DeviceCodeTimeoutError(
            AuthProvider.TWITCH,
            { 
              timeout,
              device_code: deviceCode.device_code,
              user_code: deviceCode.user_code
            }
          );
        }

        // Wait for the polling interval
        await new Promise(resolve => setTimeout(resolve, pollInterval));

        // Try to exchange the device code for an access token
        const token = await exchangeDeviceCode(
          clientId,
          deviceCode.device_code
        );

        return token;
      } catch (error) {
        // Check for authorization_pending error (user hasn't completed the flow yet)
        if (
          error instanceof Error && 
          error.message.includes('authorization_pending')
        ) {
          // Continue polling
          continue;
        }

        // Check for slow_down error (we need to increase the polling interval)
        if (
          error instanceof Error && 
          error.message.includes('slow_down')
        ) {
          // Increase polling interval by 5 seconds
          pollInterval += 5000;
          continue;
        }

        // Check for expired_token error (the device code has expired)
        if (
          error instanceof Error && 
          error.message.includes('expired_token')
        ) {
          throw new DeviceCodeTimeoutError(
            AuthProvider.TWITCH,
            { 
              originalError: error,
              device_code: deviceCode.device_code,
              user_code: deviceCode.user_code
            },
            error
          );
        }

        // For other errors, we should stop polling and report the error
        throw new AuthenticationFailedError(
          AuthProvider.TWITCH,
          `Device code authentication failed: ${error instanceof Error ? error.message : String(error)}`,
          AuthErrorCode.AUTHENTICATION_FAILED,
          { originalError: error },
          error instanceof Error ? error : undefined
        );
      }
    }
  }

  /**
   * Get user information from Twitch API
   */
  private async getUserInfo(token: AccessToken): Promise<{ id: string; login: string }> {
    try {
      const response = await fetch('https://api.twitch.tv/helix/users', {
        headers: {
          'Authorization': `Bearer ${token.accessToken}`,
          'Client-Id': token.clientId
        }
      });

      if (!response.ok) {
        throw new Error(`Failed to get user info: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();
      
      if (!data.data || !data.data[0]) {
        throw new Error('No user data returned from Twitch API');
      }

      return {
        id: data.data[0].id,
        login: data.data[0].login
      };
    } catch (error) {
      console.error('Error getting user info:', error);
      throw new AuthenticationFailedError(
        AuthProvider.TWITCH,
        `Failed to get user info: ${error instanceof Error ? error.message : String(error)}`,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { originalError: error },
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Refresh a Twitch auth token
   */
  protected async refreshTokenImplementation(
    provider: AuthProvider,
    token: AuthToken
  ): Promise<AuthResult> {
    if (provider !== AuthProvider.TWITCH) {
      throw new RefreshFailedError(
        provider,
        { reason: `Provider ${provider} is not supported by TwitchAuthService` }
      );
    }

    try {
      // Create a temporary auth provider for token refresh
      // Note: For device code flow, we pass an empty string as clientSecret
      // This satisfies the library's interface requirements without using a real secret
      const authProvider = new TwitchAuthProvider({
        clientId: process.env.TWITCH_CLIENT_ID || '',
        clientSecret: '' // Empty string to satisfy the API requirement
      });

      // Add user to auth provider
      await authProvider.addUserForToken({
        accessToken: token.accessToken,
        refreshToken: token.refreshToken!,
        expiresIn: 0, // Force a refresh
        obtainmentTimestamp: 0
      });

      // Get a new token (this will automatically refresh it)
      const newToken = await authProvider.getAccessTokenForUser('');

      // Convert to our token format
      const authToken: AuthToken = {
        accessToken: newToken.accessToken,
        refreshToken: newToken.refreshToken,
        expiresAt: Date.now() + (newToken.expiresIn * 1000),
        scope: newToken.scope,
        tokenType: 'bearer'
      };

      return {
        success: true,
        token: authToken
      };
    } catch (error) {
      throw new RefreshFailedError(
        provider,
        {
          originalError: error,
          message: error instanceof Error ? error.message : String(error)
        },
        error instanceof Error ? error : undefined
      );
    }
  }

  /**
   * Revoke a Twitch auth token
   */
  protected async revokeTokenImplementation(
    provider: AuthProvider,
    token: AuthToken
  ): Promise<void> {
    if (provider !== AuthProvider.TWITCH) {
      throw new AuthError(
        `Provider ${provider} is not supported by TwitchAuthService`,
        provider,
        AuthErrorCode.TOKEN_REVOKED,
        { supportedProvider: AuthProvider.TWITCH }
      );
    }

    try {
      // Revoke the token with Twitch
      await revokeToken(
        process.env.TWITCH_CLIENT_ID || '',
        token.accessToken
      );
    } catch (error) {
      console.error('Error revoking token:', error);
      throw new AuthError(
        `Failed to revoke token: ${error instanceof Error ? error.message : String(error)}`,
        provider,
        AuthErrorCode.TOKEN_REVOKED,
        { originalError: error },
        error instanceof Error ? error : undefined
      );
    }
  }
}

/**
 * Singleton instance of TwitchAuthService
 */
let twitchAuthServiceInstance: TwitchAuthService | null = null;

/**
 * Get the Twitch auth service instance
 */
export function getTwitchAuthService(eventBus: EventBus): TwitchAuthService {
  if (!twitchAuthServiceInstance) {
    twitchAuthServiceInstance = new TwitchAuthService(eventBus);
  }
  return twitchAuthServiceInstance;
}