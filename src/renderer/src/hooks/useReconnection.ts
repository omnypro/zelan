import { useState, useEffect } from 'react'
import type { ReconnectionOptions } from '@s/types/reconnection'

/**
 * Result type for the reconnection state
 */
export interface ReconnectionState {
  pending: boolean
  attempts: number
  lastAttempt: number
}

/**
 * Hook to access reconnection functionality for adapters
 */
export function useReconnection(adapterId: string) {
  const [reconnectionState, setReconnectionState] = useState<ReconnectionState | null>(null)
  const [options, setOptions] = useState<ReconnectionOptions | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  // Get the reconnection state for this adapter
  useEffect(() => {
    if (!window.trpc?.reconnection?.getState) {
      console.error('tRPC client or reconnection.getState not available')
      return
    }

    const fetchReconnectionState = async () => {
      try {
        const state = await window.trpc.reconnection.getState.query(adapterId)
        setReconnectionState(state)
      } catch (err) {
        console.error('Error fetching reconnection state:', err)
        setError(err instanceof Error ? err : new Error(String(err)))
      }
    }

    fetchReconnectionState()

    // Poll for reconnection state updates every 2 seconds
    const interval = setInterval(fetchReconnectionState, 2000)

    return () => {
      clearInterval(interval)
    }
  }, [adapterId])

  // Get the reconnection options
  useEffect(() => {
    if (!window.trpc?.reconnection?.getOptions) {
      console.error('tRPC client or reconnection.getOptions not available')
      return
    }

    const fetchOptions = async () => {
      try {
        const opts = await window.trpc.reconnection.getOptions.query()
        setOptions(opts)
      } catch (err) {
        console.error('Error fetching reconnection options:', err)
        setError(err instanceof Error ? err : new Error(String(err)))
      }
    }

    fetchOptions()
  }, [])

  // Function to force reconnection
  const reconnectNow = async () => {
    if (!window.trpc?.reconnection?.reconnectNow) {
      const err = new Error('tRPC client or reconnection.reconnectNow not available')
      setError(err)
      throw err
    }

    setIsLoading(true)
    setError(null)

    try {
      await window.trpc.reconnection.reconnectNow.mutate(adapterId)
      // After reconnection, refetch the state
      const state = await window.trpc.reconnection.getState.query(adapterId)
      setReconnectionState(state)
    } catch (err) {
      console.error('Error reconnecting adapter:', err)
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }

  // Function to cancel pending reconnection
  const cancelReconnection = async () => {
    if (!window.trpc?.reconnection?.cancelReconnection) {
      const err = new Error('tRPC client or reconnection.cancelReconnection not available')
      setError(err)
      throw err
    }

    setIsLoading(true)
    setError(null)

    try {
      await window.trpc.reconnection.cancelReconnection.mutate(adapterId)
      // After cancellation, refetch the state
      const state = await window.trpc.reconnection.getState.query(adapterId)
      setReconnectionState(state)
    } catch (err) {
      console.error('Error cancelling reconnection:', err)
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }

  // Function to update reconnection options
  const updateOptions = async (newOptions: Partial<ReconnectionOptions>) => {
    if (!window.trpc?.reconnection?.updateOptions) {
      const err = new Error('tRPC client or reconnection.updateOptions not available')
      setError(err)
      throw err
    }

    setIsLoading(true)
    setError(null)

    try {
      await window.trpc.reconnection.updateOptions.mutate(newOptions)
      // Refetch options after update
      const opts = await window.trpc.reconnection.getOptions.query()
      setOptions(opts)
    } catch (err) {
      console.error('Error updating reconnection options:', err)
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }

  return {
    reconnectionState,
    options,
    reconnectNow,
    cancelReconnection,
    updateOptions,
    isLoading,
    error
  }
}