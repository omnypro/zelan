import { useState, useEffect, useCallback } from 'react'
import { useTrpc } from './useTrpc'
import type { WebSocketConfig } from '../trpc/shared/types'

/**
 * Hook for managing the WebSocket server through tRPC
 */
export function useWebSocketServerTrpc() {
  const trpc = useTrpc()
  const [isRunning, setIsRunning] = useState(false)
  const [clientCount, setClientCount] = useState(0)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Fetch server status
  const fetchStatus = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available')
      setError('TRPC client not available')
      return
    }

    setIsLoading(true)
    setError(null)

    try {
      const status = await trpc.client.websocket.getStatus.query()
      setIsRunning(status.isRunning)
      setClientCount(status.clientCount)
    } catch (err) {
      console.error('Error fetching WebSocket status:', err)
      const message = err instanceof Error ? err.message : 'Failed to fetch WebSocket server status'
      setError(message)
    } finally {
      setIsLoading(false)
    }
  }, [trpc.client])

  // Start the server
  const startServer = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available')
      setError('TRPC client not available')
      return false
    }

    setIsLoading(true)
    setError(null)

    try {
      const result = await trpc.client.websocket.start.mutate()

      if (result.success) {
        await fetchStatus()
        return true
      } else {
        setError(result.error || 'Failed to start WebSocket server')
        return false
      }
    } catch (err) {
      console.error('Error starting WebSocket server:', err)
      const message = err instanceof Error ? err.message : 'Failed to start WebSocket server'
      setError(message)
      return false
    } finally {
      setIsLoading(false)
    }
  }, [trpc.client, fetchStatus])

  // Stop the server
  const stopServer = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available')
      setError('TRPC client not available')
      return false
    }

    setIsLoading(true)
    setError(null)

    try {
      const result = await trpc.client.websocket.stop.mutate()

      if (result.success) {
        await fetchStatus()
        return true
      } else {
        setError(result.error || 'Failed to stop WebSocket server')
        return false
      }
    } catch (err) {
      console.error('Error stopping WebSocket server:', err)
      const message = err instanceof Error ? err.message : 'Failed to stop WebSocket server'
      setError(message)
      return false
    } finally {
      setIsLoading(false)
    }
  }, [trpc.client, fetchStatus])

  // Update server configuration
  const updateConfig = useCallback(
    async (config: WebSocketConfig) => {
      if (!trpc.client) {
        console.error('TRPC client not available')
        setError('TRPC client not available')
        return false
      }

      setIsLoading(true)
      setError(null)

      try {
        const result = await trpc.client.websocket.updateConfig.mutate(config)

        if (result.success) {
          await fetchStatus()
          return true
        } else {
          setError(result.error || 'Failed to update WebSocket server config')
          return false
        }
      } catch (err) {
        console.error('Error updating WebSocket config:', err)
        const message =
          err instanceof Error ? err.message : 'Failed to update WebSocket server config'
        setError(message)
        return false
      } finally {
        setIsLoading(false)
      }
    },
    [trpc.client, fetchStatus]
  )

  // Auto-fetch status on mount, but only if tRPC client is available
  useEffect(() => {
    if (trpc.client) {
      fetchStatus()

      // Set up polling to keep status updated
      const intervalId = setInterval(fetchStatus, 5000)

      return () => {
        clearInterval(intervalId)
      }
    }
  }, [fetchStatus, trpc.client])

  return {
    isRunning,
    clientCount,
    isLoading,
    error,
    fetchStatus,
    startServer,
    stopServer,
    updateConfig
  }
}
