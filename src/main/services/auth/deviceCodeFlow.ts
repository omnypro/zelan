import { AccessToken, RefreshingAuthProvider } from '@twurple/auth'

// Type definitions for Device Code Flow
export interface DeviceCodeResponse {
  device_code: string
  user_code: string
  verification_uri: string
  expires_in: number
  interval: number
}

export interface TokenResponse {
  access_token: string
  refresh_token: string
  expires_in: number
  scope: string[]
  token_type: string
}

export interface DeviceCodeFlowOptions {
  clientId: string
  scopes: string[]
  onTokenRefresh?: (userId: string, token: AccessToken) => void
}

/**
 * Implementation of Twitch's Device Code OAuth flow
 * See: https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow
 */
export class DeviceCodeFlow {
  private clientId: string
  private scopes: string[]
  private onTokenRefresh?: (userId: string, token: AccessToken) => void
  private authProvider: RefreshingAuthProvider | null = null

  constructor(options: DeviceCodeFlowOptions) {
    this.clientId = options.clientId
    this.scopes = options.scopes
    this.onTokenRefresh = options.onTokenRefresh
  }

  /**
   * Start the device code flow
   * @returns Device code information to show to the user
   */
  public async getDeviceCode(): Promise<DeviceCodeResponse> {
    const params = new URLSearchParams({
      client_id: this.clientId,
      scopes: this.scopes.join(' ')
    })

    const response = await fetch(`https://id.twitch.tv/oauth2/device`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded'
      },
      body: params
    })

    if (!response.ok) {
      const errorData = await response.json()
      throw new Error(`Failed to get device code: ${errorData.message}`)
    }

    return await response.json()
  }

  /**
   * Poll for token using the device code
   * @param deviceCode The device code from getDeviceCode response
   * @param interval Polling interval in seconds
   * @param onProgress Optional callback for progress updates
   * @returns The token response
   */
  public async pollForToken(
    deviceCode: string,
    interval: number,
    onProgress?: (attempt: number, message: string) => void
  ): Promise<TokenResponse> {
    let attemptCount = 0

    return new Promise((resolve, reject) => {
      const pollInterval = setInterval(async () => {
        attemptCount++

        try {
          if (onProgress) {
            onProgress(attemptCount, 'Checking for authorization...')
          }

          const params = new URLSearchParams({
            client_id: this.clientId,
            device_code: deviceCode,
            grant_type: 'urn:ietf:params:oauth:grant-type:device_code'
          })

          const response = await fetch('https://id.twitch.tv/oauth2/token', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/x-www-form-urlencoded'
            },
            body: params
          })

          const data = await response.json()

          if (response.ok) {
            clearInterval(pollInterval)
            resolve(data)
          } else {
            // Handle known error codes
            switch (data.error) {
              case 'authorization_pending':
                // User hasn't authorized yet, continue polling
                break
              case 'slow_down':
                // We're polling too fast, increase interval
                clearInterval(pollInterval)
                reject(new Error('Polling too quickly, please try again'))
                break
              case 'expired_token':
                // Device code has expired
                clearInterval(pollInterval)
                reject(new Error('Authorization timed out, please try again'))
                break
              case 'access_denied':
                // User declined authorization
                clearInterval(pollInterval)
                reject(new Error('Authorization was denied'))
                break
              default:
                // Unknown error
                console.error('Unknown error during token polling:', data)
                // Continue polling in case of transient error
                break
            }
          }
        } catch (error) {
          console.error('Error polling for token:', error)
          // Continue polling in case of transient error
        }
      }, interval * 1000)

      // Set a timeout to stop polling after the device code expires
      // Typical expiry is 10-15 minutes
      setTimeout(
        () => {
          clearInterval(pollInterval)
          reject(new Error('Authorization timed out, please try again'))
        },
        15 * 60 * 1000
      ) // 15 minutes maximum
    })
  }

  /**
   * Create a RefreshingAuthProvider from token response
   * @param tokenData The token data from pollForToken
   * @returns RefreshingAuthProvider instance
   */
  public createAuthProvider(tokenData: TokenResponse): RefreshingAuthProvider {
    const accessToken: AccessToken = {
      accessToken: tokenData.access_token,
      refreshToken: tokenData.refresh_token,
      expiresIn: tokenData.expires_in,
      obtainmentTimestamp: Date.now(),
      scope: tokenData.scope
    }

    this.authProvider = new RefreshingAuthProvider({
      clientId: this.clientId,
      clientSecret: '' // We don't use client secret for device code flow, but it's required by the type
    })

    // Add listener for token refresh events
    this.authProvider.onRefresh((userId, newTokenData) => {
      console.log('Token refreshed for user', userId)

      // Call the onTokenRefresh callback if provided
      if (this.onTokenRefresh) {
        this.onTokenRefresh(userId, newTokenData)
      }
    })

    // We need to use any userId here, usually would be the channel ID
    // but in this case, for a device code flow, we can use any string
    this.authProvider.addUser('deviceflow', accessToken, ['chat'])

    return this.authProvider
  }
}
