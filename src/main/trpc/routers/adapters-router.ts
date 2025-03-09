import { router, procedure, adapterIdSchema, adapterConfigSchema } from '@s/trpc'
import type { TRPCContext } from '../context'
import { z } from 'zod'
import { createSerializableAdapter } from '@s/utils/rx-trpc'

export const adaptersRouter = router({
  // Get all adapters
  getAll: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    const adapters = ctx.adapterManager.getAllAdapters()
    return adapters.map(createSerializableAdapter)
  }),

  // Get adapter by id
  getById: procedure
    .input(adapterIdSchema)
    .query(({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      const adapter = ctx.adapterManager.getAdapter(input)
      if (!adapter) {
        throw new Error(`Adapter not found: ${input}`)
      }
      return createSerializableAdapter(adapter)
    }),

  // Create adapter
  create: procedure
    .input(adapterConfigSchema)
    .mutation(async ({ ctx, input }) => {
      const adapter = await ctx.adapterManager.createAdapter(input)
      return createSerializableAdapter(adapter)
    }),

  // Start adapter
  start: procedure
    .input(adapterIdSchema)
    .mutation(async ({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      await ctx.adapterManager.startAdapter(input)
      return true
    }),

  // Stop adapter
  stop: procedure
    .input(adapterIdSchema)
    .mutation(async ({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      await ctx.adapterManager.stopAdapter(input)
      return true
    }),

  // Update adapter
  update: procedure
    .input(z.object({
      id: adapterIdSchema,
      config: z.record(z.string(), z.any())
    }))
    .mutation(async ({ ctx, input }: { ctx: TRPCContext, input: { id: string, config: Record<string, any> } }) => {
      await ctx.adapterManager.updateAdapter(input.id, input.config)
      return true
    }),

  // Delete adapter
  delete: procedure
    .input(adapterIdSchema)
    .mutation(async ({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      await ctx.adapterManager.deleteAdapter(input)
      return true
    }),

  // Get available adapter types
  getTypes: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.adapterManager.getAvailableAdapterTypes()
  })
})

// Export type
export type AdaptersRouter = typeof adaptersRouter