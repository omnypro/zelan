import { initTRPC } from '@trpc/server'
import { z } from 'zod'

/**
 * tRPC initialization point with context type
 * 
 * The context will be properly typed in the main process implementation
 */
export const t = initTRPC.context<any>().create()

/**
 * Base procedure builders
 */
export const router = t.router
export const procedure = t.procedure
export const middleware = t.middleware

/**
 * Export types from the main implementation
 */
export type AppRouter = ReturnType<typeof router>

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
 * Auth input types
 */
export const authProviderSchema = z.enum(['twitch'])
export const authOptionsSchema = z.object({
  clientId: z.string(),
  scopes: z.array(z.string()).optional(),
  redirectUri: z.string().optional(),
  forceVerify: z.boolean().optional()
})
export const authenticateSchema = z.object({
  provider: authProviderSchema,
  options: authOptionsSchema
})

/**
 * Create the root router
 */
export const appRouter = router({
  // WebSocket server procedures
  websocket: router({
    // Get WebSocket server status
    getStatus: procedure.query(() => {
      // This will be implemented later in the server
      return {
        running: false,
        clientCount: 0,
        port: 8081
      }
    }),

    // Start WebSocket server
    start: procedure.mutation(() => {
      // This will be implemented later in the server
      return true
    }),

    // Stop WebSocket server
    stop: procedure.mutation(() => {
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
      .query(() => {
        // This will be implemented later in the server
        return {} as Record<string, unknown>
      }),

    // Set configuration value
    set: procedure
      .input(
        z.object({
          key: configPathSchema,
          value: configValueSchema
        })
      )
      .mutation(() => {
        // This will be implemented later in the server
        return true
      }),

    // Update multiple configuration values
    update: procedure.input(configUpdatesSchema).mutation(() => {
      // This will be implemented later in the server
      return true
    }),

    // Delete configuration key
    delete: procedure.input(configPathSchema).mutation(() => {
      // This will be implemented later in the server
      return true
    }),

    // Check if configuration key exists
    has: procedure.input(configPathSchema).query(() => {
      // This will be implemented later in the server
      return true
    }),

    // Get all configuration
    getAll: procedure.query(() => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Get configuration path
    getPath: procedure.query(() => {
      // This will be implemented later in the server
      return ''
    }),

    // Subscribe to configuration changes
    onConfigChange: procedure.subscription(() => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Subscribe to specific configuration path changes
    onPathChange: procedure.input(configPathSchema).subscription(() => {
      // This will be implemented later in the server
      return {} as any
    })
  }),

  // Event procedures
  events: router({
    // Send event to main process
    send: procedure.input(z.any()).mutation(() => {
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
    getAll: procedure.query(() => {
      // This will be implemented later in the server
      return [] as any[]
    }),

    // Get adapter by id
    getById: procedure.input(adapterIdSchema).query(() => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Create adapter
    create: procedure.input(adapterConfigSchema).mutation(() => {
      // This will be implemented later in the server
      return {} as any
    }),

    // Start adapter
    start: procedure.input(adapterIdSchema).mutation(() => {
      // This will be implemented later in the server
      return true
    }),

    // Stop adapter
    stop: procedure.input(adapterIdSchema).mutation(() => {
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
      .mutation(() => {
        // This will be implemented later in the server
        return true
      }),

    // Delete adapter
    delete: procedure.input(adapterIdSchema).mutation(() => {
      // This will be implemented later in the server
      return true
    }),

    // Get available adapter types
    getTypes: procedure.query(() => {
      // This will be implemented later in the server
      return [] as string[]
    })
  }),

  // Auth procedures
  auth: router({
    // Get auth status
    getStatus: procedure.input(authProviderSchema).query(() => {
      // This will be implemented in the server
      return {
        state: 'unauthenticated',
        provider: 'twitch',
        isAuthenticated: false
      }
    }),

    // Check if authenticated
    isAuthenticated: procedure.input(authProviderSchema).query(() => {
      // This will be implemented in the server
      return false
    }),

    // Start authentication
    authenticate: procedure.input(authenticateSchema).mutation(() => {
      // This will be implemented in the server
      return {
        success: false
      }
    }),

    // Refresh token
    refreshToken: procedure.input(authProviderSchema).mutation(() => {
      // This will be implemented in the server
      return {
        success: false
      }
    }),

    // Revoke token
    revokeToken: procedure.input(authProviderSchema).mutation(() => {
      // This will be implemented in the server
      return true
    }),

    // Subscribe to auth status changes
    onStatusChange: procedure.input(authProviderSchema).subscription(() => {
      // This will be implemented in the server
      return {} as any
    }),

    // Subscribe to device code events
    onDeviceCode: procedure.subscription(() => {
      // This will be implemented in the server
      return {} as any
    })
  })
})
