import { z } from 'zod';
import { initTRPC } from '@trpc/server';
import { 
  AdapterStatusSchema, 
  OperationResultSchema, 
  WebSocketStatusSchema, 
  AuthStateSchema,
  EventsResponseSchema,
  WebSocketConfigSchema
} from '../shared/types';

// Initialize tRPC backend
const t = initTRPC.create();

// Create procedures
const procedure = t.procedure;
const router = t.router;

// Define the router
export const appRouter = router({
  // Adapter procedures
  adapter: router({
    getStatus: procedure
      .input(z.string())
      .output(AdapterStatusSchema)
      .query(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters');
        const adapterManager = AdapterManager.getInstance();
        const adapter = adapterManager.getAdapter(input);

        if (!adapter) {
          return { status: 'not-found', isConnected: false };
        }

        return {
          status: adapter.state,
          isConnected: adapter.isConnected(),
          config: adapter.config
        };
      }),

    connect: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters');
        const adapterManager = AdapterManager.getInstance();
        const adapter = adapterManager.getAdapter(input);

        if (!adapter) {
          return { success: false, error: 'Adapter not found' };
        }

        try {
          await adapter.connect();
          return { success: true };
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          return { success: false, error: errorMessage };
        }
      }),

    disconnect: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters');
        const adapterManager = AdapterManager.getInstance();
        const adapter = adapterManager.getAdapter(input);

        if (!adapter) {
          return { success: false, error: 'Adapter not found' };
        }

        try {
          await adapter.disconnect();
          return { success: true };
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          return { success: false, error: errorMessage };
        }
      }),

    updateConfig: procedure
      .input(z.object({
        adapterId: z.string(),
        config: z.record(z.any())
      }))
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AdapterManager } = await import('../../core/adapters');
        const adapterManager = AdapterManager.getInstance();
        const adapter = adapterManager.getAdapter(input.adapterId);

        if (!adapter) {
          return { success: false, error: 'Adapter not found' };
        }

        try {
          adapter.updateConfig(input.config);
          return { success: true };
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          return { success: false, error: errorMessage };
        }
      }),
  }),

  // WebSocket procedures
  websocket: router({
    getStatus: procedure
      .output(WebSocketStatusSchema)
      .query(async () => {
        return {
          isRunning: false,
          clientCount: 0
        };
      }),

    start: procedure
      .output(OperationResultSchema)
      .mutation(async () => {
        return {
          success: false,
          error: 'WebSocket server is disabled'
        };
      }),

    stop: procedure
      .output(OperationResultSchema)
      .mutation(async () => {
        return {
          success: true
        };
      }),

    updateConfig: procedure
      .input(WebSocketConfigSchema)
      .output(OperationResultSchema)
      .mutation(async () => {
        return {
          success: false,
          error: 'WebSocket server is disabled'
        };
      }),
  }),

  // Auth procedures
  auth: router({
    getState: procedure
      .input(z.string())
      .output(AuthStateSchema)
      .query(async ({ input }) => {
        const { AuthService, AuthState } = await import('../../core/auth');
        const authService = AuthService.getInstance();

        return {
          state: authService.getAuthState(input),
          isAuthenticated: authService.getAuthState(input) === AuthState.AUTHENTICATED
        };
      }),

    authenticate: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AuthService } = await import('../../core/auth');
        const authService = AuthService.getInstance();

        try {
          await authService.authenticate(input);
          return { success: true };
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          return { success: false, error: errorMessage };
        }
      }),

    logout: procedure
      .input(z.string())
      .output(OperationResultSchema)
      .mutation(async ({ input }) => {
        const { AuthService } = await import('../../core/auth');
        const authService = AuthService.getInstance();

        try {
          await authService.logout(input);
          return { success: true };
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          return { success: false, error: errorMessage };
        }
      }),
  }),

  // Event procedures
  event: router({
    getRecentEvents: procedure
      .input(z.number().default(10))
      .output(EventsResponseSchema)
      .query(async () => {
        return {
          events: []
        };
      }),
  }),
});

// Export type definition of the API
export type AppRouter = typeof appRouter;