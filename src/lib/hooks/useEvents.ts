import { useState, useEffect, useCallback } from 'react'
import { useTrpc } from './useTrpc'
import type { Event, BaseEvent } from '@/lib/trpc/shared/types'

/**
 * Hook for working with events through tRPC
 */
export function useEvents() {
  const trpc = useTrpc()
  const [recentEvents, setRecentEvents] = useState<Event[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Fetch recent events
  const fetchRecentEvents = useCallback(
    async (count = 10) => {
      if (!trpc.client) {
        console.error('TRPC client not available')
        setError('TRPC client not available')
        return
      }

      setIsLoading(true)
      setError(null)

      try {
        // Use client.event not event directly
        const response = await trpc.client.event.getRecentEvents.query(count)
        setRecentEvents(response.events)
      } catch (err) {
        console.error('Error fetching events:', err)
        const message = err instanceof Error ? err.message : 'Failed to fetch events'
        setError(message)
      } finally {
        setIsLoading(false)
      }
    },
    [trpc.client]
  )

  // Fetch filtered events
  const fetchFilteredEvents = useCallback(
    async (options: { type?: string; source?: string; count?: number }) => {
      if (!trpc.client) {
        console.error('TRPC client not available')
        setError('TRPC client not available')
        return
      }

      setIsLoading(true)
      setError(null)

      try {
        const response = await trpc.client.event.getFilteredEvents.query(options)
        setRecentEvents(response.events)
      } catch (err) {
        console.error('Error fetching filtered events:', err)
        const message = err instanceof Error ? err.message : 'Failed to fetch filtered events'
        setError(message)
      } finally {
        setIsLoading(false)
      }
    },
    [trpc.client]
  )

  // Publish an event
  const publishEvent = useCallback(
    async (event: BaseEvent) => {
      if (!trpc.client) {
        console.error('TRPC client not available')
        setError('TRPC client not available')
        return false
      }

      setError(null)

      try {
        await trpc.client.event.publishEvent.mutate(event)
        return true
      } catch (err) {
        console.error('Error publishing event:', err)
        const message = err instanceof Error ? err.message : 'Failed to publish event'
        setError(message)
        return false
      }
    },
    [trpc.client]
  )

  // Clear events
  const clearEvents = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available')
      setError('TRPC client not available')
      return false
    }

    setError(null)

    try {
      await trpc.client.event.clearEvents.mutate()
      setRecentEvents([])
      return true
    } catch (err) {
      console.error('Error clearing events:', err)
      const message = err instanceof Error ? err.message : 'Failed to clear events'
      setError(message)
      return false
    }
  }, [trpc.client])

  // Auto-fetch recent events on mount, but only if tRPC client is available
  useEffect(() => {
    if (trpc.client) {
      fetchRecentEvents()
    }
  }, [fetchRecentEvents, trpc.client])

  return {
    events: recentEvents,
    isLoading,
    error,
    fetchRecentEvents,
    fetchFilteredEvents,
    publishEvent,
    clearEvents
  }
}
