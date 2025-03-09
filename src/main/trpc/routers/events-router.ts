import { router, procedure } from '@s/trpc'
import { observable } from '@trpc/server/observable'
import type { TRPCContext } from '../context'
import { toSerializableError } from '@s/utils/rx-trpc'
import { BaseEvent } from '@s/types/events'

export const eventsRouter = router({
  // Subscribe to events
  onEvent: procedure.subscription(({ ctx }: { ctx: TRPCContext }) => {
    // Return an observable that connects to the event bus
    return observable<BaseEvent>((emit) => {
      const logger = ctx.logger('tRPC.events')
      logger.info('Client subscribed to events')
      
      // Subscribe to all events from the event bus
      const subscription = ctx.mainEventBus.events$.subscribe({
        next: (event) => {
          // Pass the event to the client
          emit.next(event)
        },
        error: (err) => {
          logger.error('Error in event stream', {
            error: err instanceof Error ? err.message : String(err)
          })
          emit.error(err)
        }
      })
      
      // Return unsubscribe function
      return () => {
        subscription.unsubscribe()
        logger.info('Client unsubscribed from events')
      }
    })
  }),
  
  // Get recent events from cache
  getRecent: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    // Cast to any to handle potential API differences
    return (ctx.mainEventBus as any).getRecentEvents?.({
      limit: 20
    }) || []
  }),
  
  // Send event to the event bus
  send: procedure.input((val: any) => val).mutation(({ ctx, input }: { ctx: TRPCContext, input: BaseEvent }) => {
    try {
      ctx.mainEventBus.publish(input)
      return true
    } catch (error) {
      ctx.logger('tRPC.events').error('Error sending event', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw new Error('Failed to send event')
    }
  })
})

// Export type
export type EventsRouter = typeof eventsRouter