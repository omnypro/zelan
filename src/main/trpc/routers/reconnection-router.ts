import { router, procedure } from '@s/trpc'
import { z } from 'zod'
import type { TRPCContext } from '../context'

// Define a schema for reconnection options
const reconnectionOptionsSchema = z.object({
  maxAttempts: z.number().optional(),
  initialDelay: z.number().optional(),
  maxDelay: z.number().optional(),
  longIntervalDelay: z.number().optional(),
  resetCountOnSuccess: z.boolean().optional()
})

export const reconnectionRouter = router({
  // Get current reconnection state for an adapter
  getReconnectionState: procedure.input(z.string()).query(({ ctx, input }: { ctx: TRPCContext; input: string }) => {
    return ctx.reconnectionManager.getReconnectionState(input)
  }),

  // Get reconnection options
  getOptions: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.reconnectionManager.options
  }),

  // Update reconnection options
  updateOptions: procedure
    .input(reconnectionOptionsSchema)
    .mutation(({ ctx, input }: { ctx: TRPCContext; input: z.infer<typeof reconnectionOptionsSchema> }) => {
      ctx.reconnectionManager.updateOptions(input)
      return true
    }),

  // Reconnect a specific adapter now
  reconnectNow: procedure.input(z.string()).mutation(({ ctx, input }: { ctx: TRPCContext; input: string }) => {
    return ctx.reconnectionManager.reconnectNow(input)
  }),

  // Reconnect all adapters now
  reconnectAllNow: procedure.mutation(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.reconnectionManager.reconnectAllNow()
  }),

  // Cancel reconnection for an adapter
  cancelReconnection: procedure.input(z.string()).mutation(({ ctx, input }: { ctx: TRPCContext; input: string }) => {
    return ctx.reconnectionManager.cancelReconnection(input)
  })
})

// Export type
export type ReconnectionRouter = typeof reconnectionRouter
