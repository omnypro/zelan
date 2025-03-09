import { useState, useEffect, useCallback } from 'react'
import { AuthProvider, AuthState } from '@s/auth/interfaces'

// Auth status type
interface AuthStatus {
  state: AuthState
  provider: AuthProvider
  lastUpdated: number
  error?: string
  expiresAt?: number
  userId?: string
  username?: string
  isAuthenticated: boolean
}

/**
 * Auth result type
 */
interface AuthResult {
  success: boolean
  error?: string
  userId?: string
  username?: string
}

/**
 * Auth options
 */
interface AuthOptions {
  clientId: string
  scopes?: string[]
  redirectUri?: string
  forceVerify?: boolean
}

/**
 * Device code response
 */
interface DeviceCodeResponse {
  type: string
  user_code: string
  verification_uri: string
  verification_uri_complete?: string
  expires_in: number
}

/**
 * React hook for authentication
 */
export function useAuth(provider: AuthProvider = AuthProvider.TWITCH) {
  const [status, setStatus] = useState<AuthStatus>({
    state: AuthState.UNAUTHENTICATED,
    provider,
    lastUpdated: Date.now(),
    isAuthenticated: false
  })

  const [deviceCode, setDeviceCode] = useState<DeviceCodeResponse | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Get initial status
  useEffect(() => {
    if (!window.trpc) return

    // Get initial auth status
    window.trpc.auth.getStatus
      .query(provider)
      .then(setStatus)
      .catch((err) => {
        console.error('Error getting auth status:', err)
        setError(err.message)
      })

    // Subscribe to status changes
    const statusSubscription = window.trpc.auth.onStatusChange.subscribe(provider)
    const statusUnsubscribe = statusSubscription.subscribe({
      next: (updatedStatus) => {
        setStatus(updatedStatus)

        // If we get an error in the status, update the error state
        if (updatedStatus.error) {
          setError(updatedStatus.error)
        }

        // If we're no longer authenticating, we're done loading
        if (updatedStatus.state !== AuthState.AUTHENTICATING) {
          setIsLoading(false)
        }
      },
      error: (err) => {
        console.error('Error in auth status subscription:', err)
        setError(err.message)
        setIsLoading(false)
      }
    })

    // Subscribe to device code events
    const deviceCodeSubscription = window.trpc.auth.onDeviceCode.subscribe()
    const deviceCodeUnsubscribe = deviceCodeSubscription.subscribe({
      next: (response) => {
        if (response.type === 'device_code_received') {
          setDeviceCode(response)
        } else if (response.type === 'authentication_failed') {
          setError(response.error || 'Authentication failed')
          setIsLoading(false)
        }
      },
      error: (err) => {
        console.error('Error in device code subscription:', err)
        setError(err.message)
        setIsLoading(false)
      }
    })

    // Clean up subscriptions
    return () => {
      statusUnsubscribe.unsubscribe()
      deviceCodeUnsubscribe.unsubscribe()
    }
  }, [provider])

  /**
   * Start the authentication process
   */
  const login = useCallback(
    async (options: AuthOptions): Promise<AuthResult> => {
      if (!window.trpc) {
        throw new Error('tRPC client not available')
      }

      setIsLoading(true)
      setError(null)
      setDeviceCode(null)

      try {
        const result = await window.trpc.auth.authenticate.mutate({
          provider,
          options
        })

        return result
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err)
        console.error('Authentication error:', errorMessage)
        setError(errorMessage)
        setIsLoading(false)

        return {
          success: false,
          error: errorMessage
        }
      }
    },
    [provider]
  )

  /**
   * Logout/revoke the current auth token
   */
  const logout = useCallback(async (): Promise<boolean> => {
    if (!window.trpc) {
      throw new Error('tRPC client not available')
    }

    setIsLoading(true)
    setError(null)

    try {
      await window.trpc.auth.revokeToken.mutate(provider)
      return true
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      console.error('Logout error:', errorMessage)
      setError(errorMessage)
      return false
    } finally {
      setIsLoading(false)
    }
  }, [provider])

  /**
   * Refresh the auth token
   */
  const refreshToken = useCallback(async (): Promise<AuthResult> => {
    if (!window.trpc) {
      throw new Error('tRPC client not available')
    }

    setIsLoading(true)
    setError(null)

    try {
      const result = await window.trpc.auth.refreshToken.mutate(provider)
      return result
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      console.error('Token refresh error:', errorMessage)
      setError(errorMessage)

      return {
        success: false,
        error: errorMessage
      }
    } finally {
      setIsLoading(false)
    }
  }, [provider])

  return {
    status,
    deviceCode,
    isLoading,
    error,
    login,
    logout,
    refreshToken
  }
}

export default useAuth
