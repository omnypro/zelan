import { EventBus } from '@s/core/bus/EventBus'
import { BaseAuthService } from './BaseAuthService'
import { getErrorService } from '@m/services/errors'
import { getLoggingService } from '@m/services/logging'
import {
  AuthProvider,
  AuthOptions,
  AuthResult,
  AuthToken,
  DeviceCodeResponse,
  AuthState
} from '@s/auth/interfaces'
import {
  AuthError,
  AuthErrorCode,
  AuthenticationFailedError,
  DeviceCodeTimeoutError,
  RefreshFailedError
} from '@s/auth/errors'

import { AccessToken, RefreshingAuthProvider, revokeToken } from '@twurple/auth'

/**
 * Handles authentication with the Twitch API
 */
export class TwitchAuthService extends BaseAuthService {
  /**
   * Default scopes for Twitch authentication
   */
  public readonly DEFAULT_SCOPES = [
    'analytics:read:extensions',
    'analytics:read:games',
    'bits:read',
    'channel:bot',
    'channel:moderate',
    'channel:read:goals',
    'channel:read:hype_train',
    'channel:read:polls',
    'channel:read:predictions',
    'channel:read:redemptions',
    'channel:read:subscriptions',
    'channel:read:vips',
    'chat:read',
    'eventsub:version1', // Required for EventSub WebSocket
    'moderation:read',
    'moderator:read:automod_settings',
    'moderator:read:blocked_terms',
    'moderator:read:chat_settings',
    'moderator:read:chatters',
    'moderator:read:followers',
    'moderator:read:guest_star',
    'moderator:read:shield_mode',
    'user:read:blocked_users',
    'user:read:broadcast',
    'user:read:chat',
    'user:read:email',
    'user:read:follows',
    'user:read:subscriptions'
  ]

  /**
   * Create a new TwitchAuthService
   */
  constructor(eventBus: EventBus) {
    super(eventBus)
    // Override the base logger with a more specific component name
    this.logger = getLoggingService().createLogger('TwitchAuthService')
  }

  /**
   * Authenticate with Twitch using device code flow
   */
  async authenticate(provider: AuthProvider, options: AuthOptions): Promise<AuthResult> {
    if (provider !== AuthProvider.TWITCH) {
      throw new AuthenticationFailedError(
        provider,
        `Provider ${provider} is not supported by TwitchAuthService`,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { supportedProvider: AuthProvider.TWITCH }
      )
    }

    if (!this.initialized) {
      await this.initialize()
    }

    try {
      // Update status to authenticating
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATING
      })

      // Get device code
      const deviceCode = await this.getDeviceCode(options)

      // Publish device code event
      this.publishAuthEvent(provider, 'device_code_received', {
        user_code: deviceCode.user_code,
        verification_uri: deviceCode.verification_uri,
        verification_uri_complete: deviceCode.verification_uri_complete,
        expires_in: deviceCode.expires_in
      })

      // Exchange device code for token
      const token = await this.pollForAccessToken(deviceCode, options)

      // Get user information
      const userData = await this.getUserInfo(token)

      // Convert token format
      const authToken: AuthToken = {
        accessToken: token.accessToken,
        refreshToken: token.refreshToken || '',
        expiresAt: Date.now() + (token.expiresIn || 0) * 1000,
        scope: token.scope,
        tokenType: 'bearer'
      }

      // Add user data as metadata to the token before saving
      const tokenWithMetadata = {
        ...authToken,
        metadata: {
          userId: userData.id,
          username: userData.login
        }
      }

      // Save the token with user metadata
      await this.tokenManager.saveToken(provider, tokenWithMetadata)

      // Update status
      this.updateStatus(provider, {
        state: AuthState.AUTHENTICATED,
        expiresAt: authToken.expiresAt,
        userId: userData.id,
        username: userData.login
      })

      // Publish auth event
      this.publishAuthEvent(provider, 'authenticated', {
        userId: userData.id,
        username: userData.login
      })

      return {
        success: true,
        token: authToken,
        userId: userData.id,
        username: userData.login
      }
    } catch (error) {
      // Update status to error
      this.updateStatus(provider, {
        state: AuthState.ERROR,
        error: error instanceof Error ? error : new Error(String(error))
      })

      // Report the error
      getErrorService().reportError(
        error instanceof AuthError
          ? error
          : new AuthenticationFailedError(
              provider,
              String(error),
              AuthErrorCode.AUTHENTICATION_FAILED,
              { originalError: error },
              error instanceof Error ? error : undefined
            )
      )

      // Publish auth error event
      this.publishAuthEvent(provider, 'authentication_failed', {
        error: error instanceof Error ? error.message : String(error)
      })

      return {
        success: false,
        error: error instanceof Error ? error : new Error(String(error))
      }
    }
  }

  /**
   * Get a device code for authentication
   *
   * Implementation of Device Code Flow for Twitch API since Twurple
   * doesn't natively support it.
   */
  private async getDeviceCode(options: AuthOptions): Promise<DeviceCodeResponse> {
    try {
      const { clientId, scopes = this.DEFAULT_SCOPES } = options

      // Request parameters
      const params = new URLSearchParams()
      params.append('client_id', clientId)
      params.append('scope', scopes.join(' '))

      // Make request to Twitch API
      const response = await fetch('https://id.twitch.tv/oauth2/device', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded'
        },
        body: params
      })

      if (!response.ok) {
        const errorText = await response.text()
        throw new Error(`Twitch API error (${response.status}): ${errorText}`)
      }

      // Parse response
      const data = await response.json()

      return {
        device_code: data.device_code,
        user_code: data.user_code,
        verification_uri: data.verification_uri,
        verification_uri_complete: data.verification_uri_complete,
        expires_in: data.expires_in,
        interval: data.interval || 5
      }
    } catch (error) {
      this.logger.error('Error getting device code', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw new AuthenticationFailedError(
        AuthProvider.TWITCH,
        `Failed to get device code: ${error instanceof Error ? error.message : String(error)}`,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { originalError: error },
        error instanceof Error ? error : undefined
      )
    }
  }

  /**
   * Poll for access token using device code
   *
   * Implementation of Device Code Flow for Twitch API since Twurple
   * doesn't natively support it.
   */
  private async pollForAccessToken(
    deviceCode: DeviceCodeResponse,
    options: AuthOptions
  ): Promise<AccessToken> {
    const { clientId } = options
    const startTime = Date.now()
    const timeout = deviceCode.expires_in * 1000

    // Initial polling delay
    let pollInterval = (deviceCode.interval || 5) * 1000

    while (true) {
      try {
        // Check if we've timed out
        if (Date.now() - startTime > timeout) {
          throw new DeviceCodeTimeoutError(AuthProvider.TWITCH, {
            timeout,
            device_code: deviceCode.device_code,
            user_code: deviceCode.user_code
          })
        }

        // Wait for the polling interval
        await new Promise((resolve) => setTimeout(resolve, pollInterval))

        // Try to exchange the device code for an access token
        const params = new URLSearchParams()
        params.append('client_id', clientId)
        params.append('device_code', deviceCode.device_code)
        params.append('grant_type', 'urn:ietf:params:oauth:grant-type:device_code')

        this.logger.info('Polling for token', {
          clientId,
          deviceCode: deviceCode.device_code,
          grantType: 'urn:ietf:params:oauth:grant-type:device_code'
        })

        try {
          const response = await fetch('https://id.twitch.tv/oauth2/token', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/x-www-form-urlencoded'
            },
            body: params
          })

          this.logger.info('Token exchange response', { status: response.status })

          // Get the response as text first
          const responseText = await response.text()
          this.logger.info('Token exchange response body', { response: responseText })

          // Parse the JSON if there's content
          const data = responseText ? JSON.parse(responseText) : {}

          // Check for response status
          if (!response.ok) {
            // Twitch's API returns a 400 with a message instead of an error field
            // Check the specific message in the response

            // Authorization is pending (user hasn't completed the flow yet)
            if (data.message === 'authorization_pending') {
              this.logger.info('Authorization pending, continuing polling')
              continue
            }

            // Slow down (we need to increase the polling interval)
            if (data.message === 'slow_down') {
              this.logger.info('Received slow_down error, increasing polling interval', {
                newInterval: pollInterval + 5000
              })
              pollInterval += 5000
              continue
            }

            // Device code has expired
            if (data.message === 'expired_token') {
              this.logger.info('Device code has expired', { device_code: deviceCode.device_code })
              throw new DeviceCodeTimeoutError(AuthProvider.TWITCH, {
                originalError: new Error(data.message || data.status),
                device_code: deviceCode.device_code,
                user_code: deviceCode.user_code
              })
            }

            // For compatibility, also check standard error field
            if (data.error === 'authorization_pending') {
              this.logger.info('Authorization pending (error field), continuing polling')
              continue
            }

            if (data.error === 'slow_down') {
              this.logger.info(
                'Received slow_down error (error field), increasing polling interval',
                { newInterval: pollInterval + 5000 }
              )
              pollInterval += 5000
              continue
            }

            if (data.error === 'expired_token') {
              this.logger.info('Device code has expired (error field)', {
                device_code: deviceCode.device_code
              })
              throw new DeviceCodeTimeoutError(AuthProvider.TWITCH, {
                originalError: new Error(data.error_description || data.error),
                device_code: deviceCode.device_code,
                user_code: deviceCode.user_code
              })
            }

            // Other errors
            const errorMessage =
              data.message ||
              data.error_description ||
              data.error ||
              `HTTP error ${response.status}`
            this.logger.error('Token exchange error', { error: errorMessage })
            throw new Error(errorMessage)
          }

          // Success! Create an AccessToken object in a format compatible with Twurple
          this.logger.info('Token exchange successful')

          // Handle scope being either a string or an array
          let scope = data.scope
          if (typeof scope === 'string') {
            scope = scope.split(' ')
          } else if (!Array.isArray(scope)) {
            // If it's neither a string nor an array, default to an empty array
            this.logger.warn('Unexpected scope format', { scope })
            scope = []
          }

          return {
            accessToken: data.access_token,
            refreshToken: data.refresh_token,
            scope: scope,
            expiresIn: data.expires_in,
            obtainmentTimestamp: Date.now()
          }
        } catch (fetchError) {
          this.logger.error('Fetch error during token exchange', {
            error: fetchError instanceof Error ? fetchError.message : String(fetchError)
          })
          const errorMessage =
            fetchError instanceof Error ? fetchError.message : 'Unknown fetch error'
          throw new Error(`Fetch error: ${errorMessage}`)
        }
      } catch (error) {
        // If it's already a DeviceCodeTimeoutError, just rethrow it
        if (error instanceof DeviceCodeTimeoutError) {
          throw error
        }

        // For other errors, we should stop polling and report the error
        throw new AuthenticationFailedError(
          AuthProvider.TWITCH,
          `Device code authentication failed: ${error instanceof Error ? error.message : String(error)}`,
          AuthErrorCode.AUTHENTICATION_FAILED,
          { originalError: error },
          error instanceof Error ? error : undefined
        )
      }
    }
  }

  /**
   * Get user information from Twitch API
   */
  private async getUserInfo(token: {
    accessToken: string
  }): Promise<{ id: string; login: string }> {
    try {
      const response = await fetch('https://api.twitch.tv/helix/users', {
        headers: {
          Authorization: `Bearer ${token.accessToken}`,
          'Client-Id': process.env.TWITCH_CLIENT_ID || ''
        }
      })

      if (!response.ok) {
        throw new Error(`Failed to get user info: ${response.status} ${response.statusText}`)
      }

      const data = await response.json()

      if (!data.data || !data.data[0]) {
        throw new Error('No user data returned from Twitch API')
      }

      return {
        id: data.data[0].id,
        login: data.data[0].login
      }
    } catch (error) {
      this.logger.error('Error getting user info', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw new AuthenticationFailedError(
        AuthProvider.TWITCH,
        `Failed to get user info: ${error instanceof Error ? error.message : String(error)}`,
        AuthErrorCode.AUTHENTICATION_FAILED,
        { originalError: error },
        error instanceof Error ? error : undefined
      )
    }
  }

  /**
   * Refresh a Twitch auth token using Twurple's RefreshingAuthProvider
   */
  protected async refreshTokenImplementation(
    provider: AuthProvider,
    token: AuthToken
  ): Promise<AuthResult> {
    if (provider !== AuthProvider.TWITCH) {
      throw new RefreshFailedError(provider, {
        reason: `Provider ${provider} is not supported by TwitchAuthService`
      })
    }

    try {
      // Make sure we have a refresh token
      if (!token.refreshToken) {
        throw new RefreshFailedError(provider, { reason: 'No refresh token available' })
      }

      // Create a temporary auth provider for token refresh
      // For desktop apps using Device Code Flow, we can use an empty string as clientSecret
      const authProvider = new RefreshingAuthProvider({
        clientId: process.env.TWITCH_CLIENT_ID || '',
        clientSecret: '' // Empty string for Device Code Flow
      })

      // Add user to auth provider
      await authProvider.addUserForToken({
        accessToken: token.accessToken,
        refreshToken: token.refreshToken,
        expiresIn: 0, // Force a refresh
        obtainmentTimestamp: 0
      })

      // Get a new token (this will automatically refresh it)
      const newToken = await authProvider.getAnyAccessToken()

      // Convert to our token format, preserving metadata from original token
      const authToken: AuthToken = {
        accessToken: newToken.accessToken,
        refreshToken: newToken.refreshToken || '',
        expiresAt: Date.now() + (newToken.expiresIn || 0) * 1000,
        scope: newToken.scope,
        tokenType: 'bearer',
        metadata: token.metadata // Preserve metadata from the original token
      }

      return {
        success: true,
        token: authToken
      }
    } catch (error) {
      throw new RefreshFailedError(
        provider,
        {
          originalError: error,
          message: error instanceof Error ? error.message : String(error)
        },
        error instanceof Error ? error : undefined
      )
    }
  }

  /**
   * Revoke a Twitch auth token using Twurple's revokeToken function
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
      )
    }

    try {
      // Use Twurple's revokeToken function
      await revokeToken(process.env.TWITCH_CLIENT_ID || '', token.accessToken)
    } catch (error) {
      this.logger.error('Error revoking token', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw new AuthError(
        `Failed to revoke token: ${error instanceof Error ? error.message : String(error)}`,
        provider,
        AuthErrorCode.TOKEN_REVOKED,
        { originalError: error },
        error instanceof Error ? error : undefined
      )
    }
  }
}

/**
 * Singleton instance of TwitchAuthService
 */
let twitchAuthServiceInstance: TwitchAuthService | null = null

/**
 * Get the Twitch auth service instance
 */
export function getTwitchAuthService(eventBus: EventBus): TwitchAuthService {
  if (!twitchAuthServiceInstance) {
    twitchAuthServiceInstance = new TwitchAuthService(eventBus)
  }
  return twitchAuthServiceInstance
}
