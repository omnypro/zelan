import { router, procedure } from '@s/trpc'
import { observable } from '@trpc/server/observable'
import type { TRPCContext } from '../context'

export const websocketRouter = router({
  // Get WebSocket server status
  getStatus: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.webSocketService.getStatus()
  }),

  // Start WebSocket server
  start: procedure.mutation(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.webSocketService.start()
  }),

  // Stop WebSocket server
  stop: procedure.mutation(({ ctx }: { ctx: TRPCContext }) => {
    ctx.webSocketService.stop()
    return true
  })
})

// Export type
export type WebsocketRouter = typeof websocketRouter