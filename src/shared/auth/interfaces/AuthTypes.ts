/**
 * Authentication provider types
 */
export enum AuthProvider {
  TWITCH = 'twitch'
  // Add other providers as needed
}

/**
 * Authentication token structure
 */
export interface AuthToken {
  accessToken: string
  refreshToken?: string
  expiresAt: number // timestamp in ms
  scope: string[]
  tokenType: string
  metadata?: {
    userId?: string
    username?: string
    [key: string]: any
  }
}

/**
 * Authentication state
 */
export enum AuthState {
  UNAUTHENTICATED = 'unauthenticated',
  AUTHENTICATING = 'authenticating',
  AUTHENTICATED = 'authenticated',
  ERROR = 'error'
}

/**
 * Authentication status with metadata
 */
export interface AuthStatus {
  state: AuthState
  provider: AuthProvider
  lastUpdated: number
  error?: Error
  expiresAt?: number
  userId?: string
  username?: string
}

/**
 * Authentication options for different providers
 */
export interface AuthOptions {
  clientId: string
  scopes: string[]
  redirectUri?: string
  forceVerify?: boolean
}

/**
 * Authentication result
 */
export interface AuthResult {
  success: boolean
  token?: AuthToken
  error?: Error
  userId?: string
  username?: string
}

/**
 * Device code auth response
 */
export interface DeviceCodeResponse {
  device_code: string
  user_code: string
  verification_uri: string
  verification_uri_complete?: string
  expires_in: number
  interval: number
}
