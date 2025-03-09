import { router, procedure, authProviderSchema, authenticateSchema } from '@s/trpc'
import { observable } from '@trpc/server/observable'
import type { TRPCContext } from '../context'
import { AuthProvider } from '@s/auth/interfaces'

export const authRouter = router({
  // Get auth status
  getStatus: procedure
    .input(authProviderSchema)
    .query(({ ctx, input }: { ctx: TRPCContext, input: AuthProvider }) => {
      const status = ctx.authService.getStatus(input)
      return {
        state: status.state,
        provider: status.provider,
        isAuthenticated: status.state === 'authenticated'
      }
    }),

  // Check if authenticated
  isAuthenticated: procedure
    .input(authProviderSchema)
    .query(({ ctx, input }: { ctx: TRPCContext, input: AuthProvider }) => {
      return ctx.authService.isAuthenticated(input)
    }),

  // Start authentication
  authenticate: procedure
    .input(authenticateSchema)
    .mutation(async ({ ctx, input }) => {
      try {
        await ctx.authService.authenticate(input.provider, input.options)
        return { success: true }
      } catch (error) {
        ctx.logger('tRPC.auth').error('Authentication error', {
          error: error instanceof Error ? error.message : String(error),
          provider: input.provider
        })
        return { success: false, error: error instanceof Error ? error.message : String(error) }
      }
    }),

  // Refresh token
  refreshToken: procedure
    .input(authProviderSchema)
    .mutation(async ({ ctx, input }: { ctx: TRPCContext, input: AuthProvider }) => {
      try {
        await ctx.authService.refreshToken(input)
        return { success: true }
      } catch (error) {
        ctx.logger('tRPC.auth').error('Token refresh error', {
          error: error instanceof Error ? error.message : String(error),
          provider: input
        })
        return { success: false, error: error instanceof Error ? error.message : String(error) }
      }
    }),

  // Revoke token
  revokeToken: procedure
    .input(authProviderSchema)
    .mutation(async ({ ctx, input }: { ctx: TRPCContext, input: AuthProvider }) => {
      try {
        await ctx.authService.revokeToken(input)
        return true
      } catch (error) {
        ctx.logger('tRPC.auth').error('Token revocation error', {
          error: error instanceof Error ? error.message : String(error),
          provider: input
        })
        throw error
      }
    }),

  // Subscribe to auth status changes
  onStatusChange: procedure
    .input(authProviderSchema)
    .subscription(({ ctx, input }: { ctx: TRPCContext, input: AuthProvider }) => {
      return observable<any>((emit) => {
        const logger = ctx.logger('tRPC.auth')
        logger.info('Client subscribed to auth status changes', { provider: input })
        
        // Subscribe to auth status changes
        const subscription = ctx.authService.onStatusChange(input).subscribe({
          next: (status) => {
            emit.next({
              state: status.state,
              provider: status.provider,
              isAuthenticated: status.state === 'authenticated'
            })
          },
          error: (err) => {
            logger.error('Error in auth status stream', {
              error: err instanceof Error ? err.message : String(err),
              provider: input
            })
            emit.error(err)
          }
        })
        
        // Return unsubscribe function
        return () => {
          subscription.unsubscribe()
          logger.info('Client unsubscribed from auth status changes', { provider: input })
        }
      })
    }),

  // Subscribe to device code events
  onDeviceCode: procedure.subscription(({ ctx }: { ctx: TRPCContext }) => {
    return observable<any>((emit) => {
      const logger = ctx.logger('tRPC.auth')
      logger.info('Client subscribed to device code events')
      
      // Subscribe to device code events
      const subscription = ctx.authService.onDeviceCode().subscribe({
        next: (codeInfo) => {
          emit.next(codeInfo)
        },
        error: (err) => {
          logger.error('Error in device code stream', {
            error: err instanceof Error ? err.message : String(err)
          })
          emit.error(err)
        }
      })
      
      // Return unsubscribe function
      return () => {
        subscription.unsubscribe()
        logger.info('Client unsubscribed from device code events')
      }
    })
  })
})

// Export type
export type AuthRouter = typeof authRouter