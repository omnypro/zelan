import { ipcRenderer } from 'electron'
import { observable } from '@trpc/server/observable'

// Channel for tRPC requests
const TRPC_CHANNEL = 'zelan:trpc'

console.log('Initializing tRPC client with channel:', TRPC_CHANNEL)

// Map of active subscriptions
const activeSubscriptions = new Map<string, () => void>()

/**
 * Create the tRPC client
 *
 * NOTE: We're going to use our own implementation instead of the proxy client
 * since it seems to be causing issues
 */
const createClient = () => {
  // Create a simplified alternative to the proxy client
  // This is a workaround for the issue with the createTRPCProxyClient
  const makeModuleWithProcedures = (moduleName) => {
    const createProcedureCaller = (procedureName, type) => {
      return {
        query: async (input) => {
          console.log(`Calling ${moduleName}.${procedureName} (${type}) with:`, input)
          const result = await ipcRenderer.invoke(TRPC_CHANNEL, {
            id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
            type,
            path: `${moduleName}.${procedureName}`,
            input
          })

          if (result.type === 'error') {
            console.error(`Error in ${moduleName}.${procedureName}:`, result.error)
            throw new Error(result.error.message)
          }

          return result.result
        },
        mutate: async (input) => {
          console.log(`Calling ${moduleName}.${procedureName} (${type}) with:`, input)
          const result = await ipcRenderer.invoke(TRPC_CHANNEL, {
            id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
            type,
            path: `${moduleName}.${procedureName}`,
            input
          })

          if (result.type === 'error') {
            console.error(`Error in ${moduleName}.${procedureName}:`, result.error)
            throw new Error(result.error.message)
          }

          return result.result
        },
        subscribe: (input) => {
          console.log(`Subscribing to ${moduleName}.${procedureName} with:`, input)
          const id = `${Date.now()}-${Math.random().toString(36).slice(2)}`
          const subChannelName = `${TRPC_CHANNEL}:${id}`

          // Start subscription on server
          ipcRenderer.invoke(TRPC_CHANNEL, {
            id,
            type: 'subscription',
            path: `${moduleName}.${procedureName}`,
            input
          })

          // Return subscription interface
          return {
            subscribe: (observer) => {
              // Handle incoming messages
              const onMessage = (_, data) => {
                if (data.type === 'data') {
                  observer.next(data.data)
                } else if (data.type === 'error') {
                  observer.error(new Error(data.error.message))
                } else if (data.type === 'stopped') {
                  observer.complete()
                  cleanup()
                }
              }

              // Add listener
              ipcRenderer.on(subChannelName, onMessage)

              // Cleanup function
              const cleanup = () => {
                ipcRenderer.removeListener(subChannelName, onMessage)
                ipcRenderer.send(`${subChannelName}:stop`)
                activeSubscriptions.delete(id)
              }

              // Store cleanup function
              activeSubscriptions.set(id, cleanup)

              // Return unsubscribe function
              return {
                unsubscribe: cleanup
              }
            }
          }
        }
      }
    }

    return {
      // Configure which procedures each module has
      ...(moduleName === 'config'
        ? {
            get: createProcedureCaller('get', 'query'),
            set: createProcedureCaller('set', 'mutation'),
            update: createProcedureCaller('update', 'mutation'),
            delete: createProcedureCaller('delete', 'mutation'),
            has: createProcedureCaller('has', 'query'),
            getAll: createProcedureCaller('getAll', 'query'),
            getPath: createProcedureCaller('getPath', 'query'),
            onConfigChange: createProcedureCaller('onConfigChange', 'subscription'),
            onPathChange: createProcedureCaller('onPathChange', 'subscription')
          }
        : {}),

      ...(moduleName === 'events'
        ? {
            send: createProcedureCaller('send', 'mutation'),
            onEvent: createProcedureCaller('onEvent', 'subscription')
          }
        : {}),

      ...(moduleName === 'adapters'
        ? {
            getAll: createProcedureCaller('getAll', 'query'),
            getById: createProcedureCaller('getById', 'query'),
            create: createProcedureCaller('create', 'mutation'),
            start: createProcedureCaller('start', 'mutation'),
            stop: createProcedureCaller('stop', 'mutation'),
            update: createProcedureCaller('update', 'mutation'),
            delete: createProcedureCaller('delete', 'mutation'),
            getTypes: createProcedureCaller('getTypes', 'query')
          }
        : {}),

      ...(moduleName === 'websocket'
        ? {
            getStatus: createProcedureCaller('getStatus', 'query'),
            start: createProcedureCaller('start', 'mutation'),
            stop: createProcedureCaller('stop', 'mutation')
          }
        : {}),
        
      ...(moduleName === 'auth'
        ? {
            getStatus: createProcedureCaller('getStatus', 'query'),
            isAuthenticated: createProcedureCaller('isAuthenticated', 'query'),
            authenticate: createProcedureCaller('authenticate', 'mutation'),
            refreshToken: createProcedureCaller('refreshToken', 'mutation'),
            revokeToken: createProcedureCaller('revokeToken', 'mutation'),
            onStatusChange: createProcedureCaller('onStatusChange', 'subscription'),
            onDeviceCode: createProcedureCaller('onDeviceCode', 'subscription')
          }
        : {})
    }
  }

  // Return the client with modules
  return {
    config: makeModuleWithProcedures('config'),
    events: makeModuleWithProcedures('events'),
    adapters: makeModuleWithProcedures('adapters'),
    websocket: makeModuleWithProcedures('websocket'),
    auth: makeModuleWithProcedures('auth')
  }
}

// Use our custom client
export const trpcClient = createClient()

console.log('tRPC client created:', !!trpcClient)
console.log('tRPC client keys:', Object.keys(trpcClient))
console.log(
  'tRPC config keys:',
  trpcClient.config ? Object.keys(trpcClient.config) : 'No config module'
)
console.log(
  'tRPC auth keys:',
  trpcClient.auth ? Object.keys(trpcClient.auth) : 'No auth module'
)
console.log(
  'tRPC websocket keys:',
  trpcClient.websocket ? Object.keys(trpcClient.websocket) : 'No websocket module'
)

/**
 * Helper function to create an observable from a tRPC subscription
 */
export function createObservableFromSubscription<T>(subscriptionFn: () => any) {
  return observable<T>((observer) => {
    const subscription = subscriptionFn().subscribe({
      next: (data: T) => observer.next(data),
      error: (err: Error) => observer.error(err),
      complete: () => observer.complete()
    })

    return () => {
      subscription.unsubscribe()
    }
  })
}

/**
 * Clean up all active subscriptions
 */
export function cleanupSubscriptions() {
  for (const cleanup of activeSubscriptions.values()) {
    cleanup()
  }
  activeSubscriptions.clear()
}

// Clean up subscriptions when the window is unloaded
window.addEventListener('unload', cleanupSubscriptions)
