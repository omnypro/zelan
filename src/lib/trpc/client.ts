import { createTRPCProxyClient, TRPCClientError, type TRPCLink } from '@trpc/client';
import { observable } from '@trpc/server/observable';
import type { AppRouter } from '../../../electron/trpc/server/router';

// Create tRPC client with a custom IPC-based link
export const client = createTRPCProxyClient<AppRouter>({
  links: [
    // Fixed the type to properly satisfy TRPCLink<AppRouter>
    (() => {
      // Return the link handler
      return ({ op, next }) => {
        // Only execute if we have window and trpcBridge available
        if (typeof window === 'undefined' || !window.trpcBridge) {
          return next(op)
        }

        // Extract the necessary information from the operation
        const { path, input, type } = op

        // Make sure type is supported by the bridge
        if (type !== 'query' && type !== 'mutation') {
          return next(op)
        }

        // Create a proper Observable to satisfy the return type expected by tRPC
        return observable((observer) => {
          (async () => {
            try {
              // Make request through our bridge
              console.log(`tRPC ${type} request: ${path}`, { input })
              if (!window.trpcBridge) {
                throw new Error('trpcBridge not available')
              }

              const response = await window.trpcBridge.request(path, type, input)
              console.log(`tRPC ${type} response:`, response)

              // Handle error responses
              if (response && typeof response === 'object' && 'error' in response) {
                const errorObj = response.error;
                const errorMessage = typeof errorObj === 'object' && errorObj !== null && 'message' in errorObj 
                  ? String((errorObj as any).message) 
                  : String(errorObj);
                
                const error = new Error(errorMessage);
                observer.error(
                  TRPCClientError.from(error)
                )
                return
              }

              // Success! Send data to the observer
              observer.next({ result: { data: response } })
              observer.complete()
            } catch (err) {
              console.error('tRPC transport error:', err)
              // Something went wrong during the request
              const error = err instanceof Error ? err : new Error(String(err));
              observer.error(TRPCClientError.from(error))
            }
          })()
        })
      }
    }) as TRPCLink<AppRouter>
  ]
})
