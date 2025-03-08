import { initTRPC } from '@trpc/server'
import { z } from 'zod'
import { AppConfig, ConfigChangeEvent } from '@s/core/config'

/**
 * tRPC initialization point
 */
export const t = initTRPC.create()

/**
 * Base procedure builders
 */
export const router = t.router
export const procedure = t.procedure
export const middleware = t.middleware

/**
 * Export types
 */
export type AppRouter = typeof appRouter

/**
 * Configuration input types using Zod
 */
export const configPathSchema = z.string()
export const configValueSchema = z.any()
export const configUpdatesSchema = z.record(z.string(), z.any())

/**
 * Adapter input types
 */
export const adapterIdSchema = z.string()
export const adapterConfigSchema = z.object({
  id: z.string(),
  type: z.string(),
  name: z.string(),
  enabled: z.boolean(),
  options: z.record(z.string(), z.any()).optional()
})

/**
 * Create the root router
 */
export const appRouter = router({
  // WebSocket server procedures
  websocket: router({
    // Get WebSocket server status
    getStatus: procedure.query(({ ctx }) => {
      // This will be implemented later in the server
      return {
        running: false,
        clientCount: 0,
        port: 8081
      }
    }),

    // Start WebSocket server
    start: procedure.mutation(({ ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Stop WebSocket server
    stop: procedure.mutation(({ ctx }) => {
      // This will be implemented later in the server
      return true
    })
  }),

  // Configuration procedures
  config: router({
    // Get configuration value
    get: procedure
      .input(
        z.object({
          key: configPathSchema,
          defaultValue: configValueSchema.optional()
        })
      )
      .query(({ input, ctx }) => {
        // This will be implemented later in the server
        return {} as any
      }),

    // Set configuration value
    set: procedure
      .input(
        z.object({
          key: configPathSchema,
          value: configValueSchema
        })
      )
      .mutation(({ input, ctx }) => {
        // This will be implemented later in the server
        return true
      }),

    // Update multiple configuration values
    update: procedure.input(configUpdatesSchema).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Delete configuration key
    delete: procedure.input(configPathSchema).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Check if configuration key exists
    has: procedure.input(configPathSchema).query(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Get all configuration
    getAll: procedure.query(({ ctx }) => {
      // This will be implemented later in the server
      return {} as AppConfig
    }),

    // Get configuration path
    getPath: procedure.query(({ ctx }) => {
      // This will be implemented later in the server
      return ''
    }),

    // Subscribe to configuration changes
    onConfigChange: procedure.subscription(() => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Subscribe to specific configuration path changes
    onPathChange: procedure.input(configPathSchema).subscription(({ input }) => {
      // This will be implemented later in the server
      return {} as any
    })
  }),

  // Event procedures
  events: router({
    // Send event to main process
    send: procedure.input(z.any()).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Subscribe to events
    onEvent: procedure.subscription(() => {
      // This will be implemented later in the server
      return {} as any
    })
  }),

  // Adapter procedures
  adapters: router({
    // Get all adapters
    getAll: procedure.query(({ ctx }) => {
      // This will be implemented later in the server
      return [] as any[]
    }),

    // Get adapter by id
    getById: procedure.input(adapterIdSchema).query(({ input, ctx }) => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Create adapter
    create: procedure.input(adapterConfigSchema).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Start adapter
    start: procedure.input(adapterIdSchema).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Stop adapter
    stop: procedure.input(adapterIdSchema).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Update adapter configuration
    update: procedure
      .input(
        z.object({
          id: adapterIdSchema,
          config: z.record(z.string(), z.any())
        })
      )
      .mutation(({ input, ctx }) => {
        // This will be implemented later in the server
        return true
      }),

    // Delete adapter
    delete: procedure.input(adapterIdSchema).mutation(({ input, ctx }) => {
      // This will be implemented later in the server
      return true
    }),

    // Get available adapter types
    getTypes: procedure.query(({ ctx }) => {
      // This will be implemented later in the server
      return [] as string[]
    })
  })
})
