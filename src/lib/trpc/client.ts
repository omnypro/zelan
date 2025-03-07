import { createTRPCProxyClient, TRPCClientError } from '@trpc/client';
import type { AppRouter } from './server';

// Create tRPC client with a custom IPC-based link
export const client = createTRPCProxyClient<AppRouter>({
  links: [
    () => {
      // Return the link handler
      return ({ op, next }) => {
        // Only execute if we have window and trpcBridge available
        if (typeof window === 'undefined' || !window.trpcBridge) {
          return next(op);
        }
        
        // Extract the necessary information from the operation
        const { path, input, type } = op;
        
        // Send the request through the bridge
        return {
          async subscribe(observer) {
            try {
              // Make request through our bridge
              console.log(`tRPC ${type} request: ${path}`, { input });
              const response = await window.trpcBridge.request(path, type, input);
              console.log(`tRPC ${type} response:`, response);
              
              // Handle error responses
              if (response && typeof response === 'object' && 'error' in response) {
                observer.error(
                  TRPCClientError.from({
                    ...response,
                    path,
                    type,
                    input,
                  })
                );
                return;
              }
              
              // Success! Send data to the observer
              observer.next({ result: { data: response } });
              observer.complete();
            } catch (err) {
              console.error('tRPC transport error:', err);
              // Something went wrong during the request
              observer.error(
                TRPCClientError.from({
                  error: {
                    message: err instanceof Error ? err.message : String(err),
                    code: 'CLIENT_ERROR',
                  },
                  path,
                  type,
                  input,
                })
              );
            }
          },
        };
      };
    },
  ],
});