import { RefreshingAuthProvider } from '@twurple/auth'
import { BehaviorSubject, Observable } from 'rxjs'
import { TokenStorage } from './tokenStorage'
import { DeviceCodeFlow, DeviceCodeResponse } from './deviceCodeFlow'

export interface AuthState {
  isAuthorized: boolean
  isAuthenticating: boolean
  authUrl?: string
  userCode?: string
  error?: string
}

export class TwitchAuthService {
  private tokenStorage: TokenStorage
  private deviceCodeFlow: DeviceCodeFlow
  private authProvider: RefreshingAuthProvider | null = null
  private userId = 'deviceflow' // We use a fixed ID for our device flow user

  // Observable state
  private authState = new BehaviorSubject<AuthState>({
    isAuthorized: false,
    isAuthenticating: false
  })

  constructor(clientId: string, scopes: string[] = ['chat:read']) {
    this.tokenStorage = new TokenStorage()

    // Initialize device code flow with token storage callback
    this.deviceCodeFlow = new DeviceCodeFlow({
      clientId,
      scopes,
      onTokenRefresh: (userId, token): void => {
        // Save refreshed tokens to storage
        this.tokenStorage.saveToken(userId, token)
      }
    })

    // Check if we have a valid token on startup
    this.checkExistingAuth()
  }

  /**
   * Get the current auth state as an observable
   */
  public get state(): Observable<AuthState> {
    return this.authState.asObservable()
  }

  /**
   * Get the current auth provider
   * @returns The auth provider or null if not authenticated
   */
  public getAuthProvider(): RefreshingAuthProvider | null {
    return this.authProvider
  }

  /**
   * Check if we have an existing valid auth token
   */
  private checkExistingAuth(): void {
    const hasValidToken = this.tokenStorage.hasValidToken(this.userId)

    if (hasValidToken) {
      const token = this.tokenStorage.getToken(this.userId)
      if (token) {
        this.authProvider = new RefreshingAuthProvider({
          clientId: process.env.TWITCH_CLIENT_ID as string,
          clientSecret: '' // Required by type, but not used for device code flow
        })

        // Set up token refresh listener to save refreshed tokens
        this.authProvider.onRefresh((userId, newTokenData) => {
          console.log('Token refreshed for existing user:', userId)
          this.tokenStorage.saveToken(userId, newTokenData)
        })

        this.authProvider.addUser(this.userId, token, ['chat'])

        this.authState.next({
          isAuthorized: true,
          isAuthenticating: false
        })
      }
    }
  }

  /**
   * Start the device code flow authentication
   */
  public async startAuth(): Promise<DeviceCodeResponse> {
    try {
      // Update state to authenticating
      this.authState.next({
        isAuthorized: false,
        isAuthenticating: true
      })

      // Get device code
      const deviceCode = await this.deviceCodeFlow.getDeviceCode()

      // Update state with auth URL and user code
      this.authState.next({
        isAuthorized: false,
        isAuthenticating: true,
        authUrl: deviceCode.verification_uri,
        userCode: deviceCode.user_code
      })

      // Start polling for token in the background
      this.pollForToken(deviceCode.device_code, deviceCode.interval)

      return deviceCode
    } catch (error) {
      console.error('Error starting auth:', error)
      this.authState.next({
        isAuthorized: false,
        isAuthenticating: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      })
      throw error
    }
  }

  /**
   * Poll for token in the background
   */
  private async pollForToken(deviceCode: string, interval: number): Promise<void> {
    try {
      const tokenData = await this.deviceCodeFlow.pollForToken(
        deviceCode,
        interval,
        (attempt, message) => {
          console.log(`Attempt ${attempt}: ${message}`)
        }
      )

      // Create auth provider
      this.authProvider = this.deviceCodeFlow.createAuthProvider(tokenData)

      // Save token
      this.tokenStorage.saveToken(this.userId, {
        accessToken: tokenData.access_token,
        refreshToken: tokenData.refresh_token,
        expiresIn: tokenData.expires_in,
        obtainmentTimestamp: Date.now(),
        scope: tokenData.scope
      })

      // Update state
      this.authState.next({
        isAuthorized: true,
        isAuthenticating: false
      })
    } catch (error) {
      console.error('Error polling for token:', error)
      this.authState.next({
        isAuthorized: false,
        isAuthenticating: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      })
    }
  }

  /**
   * Log out the current user
   */
  public logout(): void {
    // Remove token from storage
    this.tokenStorage.removeToken(this.userId)

    // Clear auth provider
    this.authProvider = null

    // Update state
    this.authState.next({
      isAuthorized: false,
      isAuthenticating: false
    })
  }
}
