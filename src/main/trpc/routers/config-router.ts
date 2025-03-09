import { router, procedure, configPathSchema, configValueSchema, configUpdatesSchema } from '@s/trpc'
import { observable } from '@trpc/server/observable'
import type { TRPCContext } from '../context'
import { z } from 'zod'

export const configRouter = router({
  // Get configuration value
  get: procedure
    .input(z.object({
      key: configPathSchema,
      defaultValue: configValueSchema.optional()
    }))
    .query(({ ctx, input }: { ctx: TRPCContext, input: { key: string, defaultValue?: any } }) => {
      return ctx.configStore.get(input.key, input.defaultValue)
    }),

  // Set configuration value
  set: procedure
    .input(z.object({
      key: configPathSchema,
      value: configValueSchema
    }))
    .mutation(({ ctx, input }: { ctx: TRPCContext, input: { key: string, value: any } }) => {
      ctx.configStore.set(input.key, input.value)
      return true
    }),

  // Update multiple configuration values
  update: procedure
    .input(configUpdatesSchema)
    .mutation(({ ctx, input }: { ctx: TRPCContext, input: Record<string, any> }) => {
      ctx.configStore.update(input)
      return true
    }),

  // Delete configuration key
  delete: procedure
    .input(configPathSchema)
    .mutation(({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      ctx.configStore.delete(input)
      return true
    }),

  // Check if configuration key exists
  has: procedure
    .input(configPathSchema)
    .query(({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      return ctx.configStore.has(input)
    }),

  // Get all configuration
  getAll: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.configStore.getAll()
  }),

  // Get configuration path
  getPath: procedure.query(({ ctx }: { ctx: TRPCContext }) => {
    return ctx.configStore.fileName
  }),

  // Subscribe to configuration changes
  onConfigChange: procedure.subscription(({ ctx }: { ctx: TRPCContext }) => {
    return observable<any>((emit) => {
      const logger = ctx.logger('tRPC.config')
      logger.info('Client subscribed to config changes')
      
      // Subscribe to all config changes
      const subscription = ctx.configStore.changes$().subscribe({
        next: (change) => {
          emit.next(change)
        },
        error: (err) => {
          logger.error('Error in config stream', {
            error: err instanceof Error ? err.message : String(err)
          })
          emit.error(err)
        }
      })
      
      // Return unsubscribe function
      return () => {
        subscription.unsubscribe()
        logger.info('Client unsubscribed from config changes')
      }
    })
  }),

  // Subscribe to specific path changes
  onPathChange: procedure
    .input(configPathSchema)
    .subscription(({ ctx, input }: { ctx: TRPCContext, input: string }) => {
      return observable<any>((emit) => {
        const logger = ctx.logger('tRPC.config')
        logger.info('Client subscribed to config path change', { path: input })
        
        // Subscribe to specific path changes
        const subscription = ctx.configStore.pathChanges$(input).subscribe({
          next: (change) => {
            emit.next(change)
          },
          error: (err) => {
            logger.error('Error in config path stream', {
              error: err instanceof Error ? err.message : String(err),
              path: input
            })
            emit.error(err)
          }
        })
        
        // Return unsubscribe function
        return () => {
          subscription.unsubscribe()
          logger.info('Client unsubscribed from config path change', { path: input })
        }
      })
    })
})

// Export type
export type ConfigRouter = typeof configRouter