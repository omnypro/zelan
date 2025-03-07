import { z } from 'zod'
import { initTRPC } from '@trpc/server'
import {
  AdapterStatusSchema,
  OperationResultSchema,
  WebSocketStatusSchema,
  AuthStateSchema,
  EventsResponseSchema,
  WebSocketConfigSchema,
  EventFilterSchema,
  BaseEventSchema,
  ConfigResponseSchema
} from '../shared/types'

// Initialize tRPC backend
const t = initTRPC.create()

// Create procedures
const procedure = t.procedure
const router = t.router

// Define the router
export const appRouter = router({
  // Config procedures
  config: router({
    // Get adapter settings
    getAdapterSettings: procedure
      .input(z.string())
      .output(ConfigResponseSchema)
      .query(async ({ input }) => {
        const { AdapterSettingsStore } = await import('../../store')
        const settingsStore = AdapterSettingsStore.getInstance()
        const settings = settingsStore.getSettings(input)
        
        return {
          success: !!settings,
          data: settings || {},
          error: settings ? undefined : 'Settings not found'
        }
      }),
      
    // Update adapter settings
    updateAdapterSettings: procedure
      .input(z.object({
        adapterId: z.string(),
        settings: z.record(z.any())
      }))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { AdapterSettingsStore } = await import('../../store')
          const settingsStore = AdapterSettingsStore.getInstance()
          settingsStore.updateSettings(input.adapterId, input.settings)
          
          // Update adapter runtime config if connected
          const { AdapterManager } = await import('../../core/adapters')
          const adapterManager = AdapterManager.getInstance()
          const adapter = adapterManager.getAdapter(input.adapterId)
          
          if (adapter) {
            adapter.updateConfig(input.settings)
          }
          
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
    
    // Get all adapter settings
    getAllAdapterSettings: procedure
      .output(ConfigResponseSchema)
      .query(async () => {
        const { AdapterSettingsStore } = await import('../../store')
        const settingsStore = AdapterSettingsStore.getInstance()
        const settings = settingsStore.getAllSettings()
        
        return {
          success: true,
          data: settings
        }
      }),
    
    // Enable/disable adapter
    setAdapterEnabled: procedure
      .input(z.object({
        adapterId: z.string(),
        enabled: z.boolean()
      }))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { AdapterSettingsStore } = await import('../../store')
          const settingsStore = AdapterSettingsStore.getInstance()
          settingsStore.setAdapterEnabled(input.adapterId, input.enabled)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
    
    // Set adapter auto-connect
    setAdapterAutoConnect: procedure
      .input(z.object({
        adapterId: z.string(),
        autoConnect: z.boolean()
      }))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { AdapterSettingsStore } = await import('../../store')
          const settingsStore = AdapterSettingsStore.getInstance()
          settingsStore.setAdapterAutoConnect(input.adapterId, input.autoConnect)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
      
    // Get global app config
    getAppConfig: procedure
      .output(ConfigResponseSchema)
      .query(async () => {
        const { ConfigStore } = await import('../../store')
        const configStore = ConfigStore.getInstance()
        const config = configStore.getConfig()
        
        return {
          success: true,
          data: config
        }
      }),
      
    // Update global app config
    updateAppConfig: procedure
      .input(z.record(z.any()))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { ConfigStore } = await import('../../store')
          const configStore = ConfigStore.getInstance()
          configStore.updateConfig(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
      
    // Get user data/preferences
    getUserData: procedure
      .output(ConfigResponseSchema)
      .query(async () => {
        const { UserDataStore } = await import('../../store')
        const userDataStore = UserDataStore.getInstance()
        const userData = userDataStore.getData()
        
        return {
          success: true,
          data: userData
        }
      }),
      
    // Update user data/preferences
    updateUserData: procedure
      .input(z.record(z.any()))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { UserDataStore } = await import('../../store')
          const userDataStore = UserDataStore.getInstance()
          userDataStore.updateData(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
      
    // Token management procedures
    getToken: procedure
      .input(z.string())
      .output(ConfigResponseSchema)
      .query(async ({ input }) => {
        const { TokenStore } = await import('../../store')
        const tokenStore = TokenStore.getInstance()
        const token = tokenStore.getToken(input)
        
        return {
          success: !!token,
          data: token || {},
          error: token ? undefined : 'Token not found'
        }
      }),
      
    saveToken: procedure
      .input(z.object({
        serviceId: z.string(),
        token: z.object({
          accessToken: z.string(),
          refreshToken: z.string().optional(),
          expiresAt: z.number(),
          scope: z.string().optional(),
          tokenType: z.string().default('Bearer'),
        })
      }))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { TokenStore } = await import('../../store')
          const tokenStore = TokenStore.getInstance()
          tokenStore.saveToken(input.serviceId, input.token)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
      
    deleteToken: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { TokenStore } = await import('../../store')
          const tokenStore = TokenStore.getInstance()
          tokenStore.deleteToken(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),
      
    hasValidToken: procedure
      .input(z.string())
      .output(ConfigResponseSchema)
      .query(async ({ input }) => {
        const { TokenStore } = await import('../../store')
        const tokenStore = TokenStore.getInstance()
        const isValid = tokenStore.hasValidToken(input)
        
        return {
          success: true,
          data: { isValid }
        }
      }),
      
    clearAllTokens: procedure
      .output(OperationResultSchema)
      .mutation(async () => {
        try {
          const { TokenStore } = await import('../../store')
          const tokenStore = TokenStore.getInstance()
          tokenStore.clearAllTokens()
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      })
  }),
  
  // Adapter procedures
  adapter: router({
    getStatus: procedure
      .input(z.string())
      .output(AdapterStatusSchema)
      .query(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters')
        const adapterManager = AdapterManager.getInstance()
        const adapter = adapterManager.getAdapter(input)

        if (!adapter) {
          return { status: 'not-found', isConnected: false }
        }

        return {
          status: adapter.state,
          isConnected: adapter.isConnected(),
          config: adapter.config
        }
      }),

    connect: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters')
        const adapterManager = AdapterManager.getInstance()
        const adapter = adapterManager.getAdapter(input)

        if (!adapter) {
          return { success: false, error: 'Adapter not found' }
        }

        try {
          await adapter.connect()
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),

    disconnect: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters')
        const adapterManager = AdapterManager.getInstance()
        const adapter = adapterManager.getAdapter(input)

        if (!adapter) {
          return { success: false, error: 'Adapter not found' }
        }

        try {
          await adapter.disconnect()
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),

    updateConfig: procedure
      .input(
        z.object({
          adapterId: z.string(),
          config: z.record(z.any())
        })
      )
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters')
        const adapterManager = AdapterManager.getInstance()
        const adapter = adapterManager.getAdapter(input.adapterId)

        if (!adapter) {
          return { success: false, error: 'Adapter not found' }
        }

        try {
          adapter.updateConfig(input.config)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      })
  }),

  // WebSocket procedures
  websocket: router({
    getStatus: procedure.output(WebSocketStatusSchema).query(async () => {
      const { WebSocketServer } = await import('../../core/websocket')
      const wsServer = WebSocketServer.getInstance()

      return {
        isRunning: wsServer.isRunning(),
        clientCount: wsServer.getClientCount()
      }
    }),

    start: procedure.output(OperationResultSchema).mutation(async () => {
      try {
        const { WebSocketServer } = await import('../../core/websocket')
        const wsServer = WebSocketServer.getInstance()
        wsServer.start()
        return { success: true }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        return { success: false, error: errorMessage }
      }
    }),

    stop: procedure.output(OperationResultSchema).mutation(async () => {
      try {
        const { WebSocketServer } = await import('../../core/websocket')
        const wsServer = WebSocketServer.getInstance()
        wsServer.stop()
        return { success: true }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        return { success: false, error: errorMessage }
      }
    }),

    updateConfig: procedure
      .input(WebSocketConfigSchema)
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { WebSocketServer } = await import('../../core/websocket')
          const wsServer = WebSocketServer.getInstance()
          wsServer.updateConfig(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      })
  }),

  // Auth procedures
  auth: router({
    getState: procedure
      .input(z.string())
      .output(AuthStateSchema)
      .query(async ({ input }) => {
        const { AuthService, AuthState } = await import('../../core/auth')
        const authService = AuthService.getInstance()

        return {
          state: authService.getAuthState(input),
          isAuthenticated: authService.getAuthState(input) === AuthState.AUTHENTICATED
        }
      }),

    authenticate: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AuthService } = await import('../../core/auth')
        const authService = AuthService.getInstance()

        try {
          await authService.authenticate(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),

    logout: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AuthService } = await import('../../core/auth')
        const authService = AuthService.getInstance()

        try {
          await authService.logout(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      })
  }),

  // Event procedures
  event: router({
    getRecentEvents: procedure
      .input(z.number().default(10))
      .output(EventsResponseSchema)
      .query(async ({ input }) => {
        const { EventCache } = await import('../../core/events')
        const eventCache = EventCache.getInstance()

        return {
          events: eventCache.getRecentEvents(input)
        }
      }),

    getFilteredEvents: procedure
      .input(EventFilterSchema)
      .output(EventsResponseSchema)
      .query(async ({ input }) => {
        const { EventCache } = await import('../../core/events')
        const eventCache = EventCache.getInstance()

        return {
          events: eventCache.filterEvents(input)
        }
      }),

    publishEvent: procedure
      .input(
        BaseEventSchema.extend({
          data: z.record(z.any()).optional()
        })
      )
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        try {
          const { EventBus } = await import('../../core/events')
          const eventBus = EventBus.getInstance()
          eventBus.publish(input)
          return { success: true }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return { success: false, error: errorMessage }
        }
      }),

    clearEvents: procedure.output(OperationResultSchema).mutation(async () => {
      try {
        const { EventCache } = await import('../../core/events')
        const eventCache = EventCache.getInstance()
        eventCache.clear()
        return { success: true }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        return { success: false, error: errorMessage }
      }
    })
  })
})

// Export type definition of the API
export type AppRouter = typeof appRouter