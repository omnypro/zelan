import { ipcMain } from 'electron'
import { ConfigStore } from '@s/core/config'
import { MainEventBus } from '@m/services/eventBus'
import { AdapterManager } from '@m/services/adapters'
import { AuthService } from '@m/services/auth'
import { Subject } from 'rxjs'
import { filter } from 'rxjs/operators'
import {
  createSubscriptionHandler,
  toSerializableError,
  createSerializableAdapter
} from '@s/utils/rx-trpc'

// Import WebSocketService
import { WebSocketService } from '@m/services/websocket'

// Import auth types
import { AuthProvider, AuthOptions, AuthState } from '@s/auth/interfaces'
import { EventCategory } from '@s/types/events'

/**
 * Context for tRPC procedures
 */
interface TRPCContext {
  mainEventBus: MainEventBus
  adapterManager: AdapterManager
  configStore: ConfigStore
  webSocketService: WebSocketService
  authService: AuthService
  senderIds: Set<number>
}

/**
 * Create the context for procedure resolvers
 */
function createContext(
  mainEventBus: MainEventBus,
  adapterManager: AdapterManager,
  configStore: ConfigStore,
  authService: AuthService
): TRPCContext {
  return {
    mainEventBus,
    adapterManager,
    configStore,
    webSocketService: WebSocketService.getInstance(mainEventBus, configStore),
    authService,
    senderIds: new Set<number>()
  }
}

/**
 * Setup tRPC server over Electron IPC
 */
export function setupTRPCServer(
  mainEventBus: MainEventBus,
  adapterManager: AdapterManager,
  configStore: ConfigStore,
  authService: AuthService
) {
  const ctx = createContext(mainEventBus, adapterManager, configStore, authService)

  // Channel for tRPC requests
  const TRPC_CHANNEL = 'zelan:trpc'

  console.log('Setting up tRPC server with channel:', TRPC_CHANNEL)

  // Set up IPC handler for tRPC requests
  ipcMain.handle(TRPC_CHANNEL, async (event, opts) => {
    const { id, type, path, input } = opts

    console.log(`Received tRPC request: ${type} ${path}`, { id, input })

    try {
      // Track sender id for subscription management
      const senderId = event.sender.id

      // Track the sender for cleanup
      if (!ctx.senderIds.has(senderId)) {
        ctx.senderIds.add(senderId)

        // Clean up when window is closed
        event.sender.once('destroyed', () => {
          ctx.senderIds.delete(senderId)
        })
      }

      // Parse the path
      const [moduleName, procedureName] = path.split('.')

      if (!moduleName || !procedureName) {
        throw new Error(`Invalid path format: ${path}. Expected "module.procedure"`)
      }

      console.log(`Processing ${moduleName}.${procedureName} (${type})`)

      // Handle the request directly based on module and procedure
      if (moduleName === 'config') {
        if (type === 'query') {
          if (procedureName === 'get') {
            const result = ctx.configStore.get(input.key, input.defaultValue)
            return { id, result, type: 'data' }
          } else if (procedureName === 'getAll') {
            const result = ctx.configStore.getAll()
            return { id, result, type: 'data' }
          } else if (procedureName === 'has') {
            const result = ctx.configStore.has(input)
            return { id, result, type: 'data' }
          } else if (procedureName === 'getPath') {
            const result = ctx.configStore.fileName
            return { id, result, type: 'data' }
          }
        } else if (type === 'mutation') {
          if (procedureName === 'set') {
            ctx.configStore.set(input.key, input.value)
            return { id, result: true, type: 'data' }
          } else if (procedureName === 'update') {
            ctx.configStore.update(input)
            return { id, result: true, type: 'data' }
          } else if (procedureName === 'delete') {
            ctx.configStore.delete(input)
            return { id, result: true, type: 'data' }
          }
        } else if (type === 'subscription') {
          const subChannelName = `${TRPC_CHANNEL}:${id}`

          if (procedureName === 'onConfigChange') {
            // Create a destroyer subject for cleanup
            const destroy$ = new Subject<void>()

            // Use our subscription helper for cleaner code
            const subscription = createSubscriptionHandler(
              ctx.configStore.changes$(),
              // Data handler
              (data) => {
                if (!event.sender.isDestroyed()) {
                  event.sender.send(subChannelName, { id, data, type: 'data' })
                }
              },
              // Error handler
              (err) => {
                if (!event.sender.isDestroyed()) {
                  event.sender.send(subChannelName, {
                    id,
                    error: toSerializableError(err),
                    type: 'error'
                  })
                }
              }
            )

            // Set up cleanup
            ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
              destroy$.next()
              destroy$.complete()
              subscription.unsubscribe()
            })

            return { id, type: 'started' }
          } else if (procedureName === 'onPathChange') {
            const path = input
            const subscription = ctx.configStore.changes$().subscribe({
              next: (data) => {
                if (data.key === path || data.key.startsWith(`${path}.`)) {
                  if (!event.sender.isDestroyed()) {
                    event.sender.send(subChannelName, {
                      id,
                      data,
                      type: 'data'
                    })
                  }
                }
              },
              error: (err) => {
                if (!event.sender.isDestroyed()) {
                  event.sender.send(subChannelName, {
                    id,
                    error: toSerializableError(err),
                    type: 'error'
                  })
                }
              }
            })

            // Set up cleanup
            ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
              subscription.unsubscribe()
            })

            return { id, type: 'started' }
          }
        }
      } else if (moduleName === 'events') {
        if (type === 'mutation' && procedureName === 'send') {
          ctx.mainEventBus.publish(input)
          return { id, result: true, type: 'data' }
        } else if (type === 'subscription' && procedureName === 'onEvent') {
          const subChannelName = `${TRPC_CHANNEL}:${id}`

          const subscription = ctx.mainEventBus.events$.subscribe({
            next: (data) => {
              if (!event.sender.isDestroyed()) {
                event.sender.send(subChannelName, {
                  id,
                  data,
                  type: 'data'
                })
              }
            },
            error: (err) => {
              if (!event.sender.isDestroyed()) {
                event.sender.send(subChannelName, {
                  id,
                  error: toSerializableError(err),
                  type: 'error'
                })
              }
            }
          })

          // Set up cleanup
          ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
            subscription.unsubscribe()
          })

          return { id, type: 'started' }
        } else if (type === 'query' && procedureName === 'getRecent') {
          const result = (ctx.mainEventBus as any).getRecentEvents?.(input) || []
          return { id, result, type: 'data' }
        }
      } else if (moduleName === 'websocket') {
        if (type === 'query' && procedureName === 'getStatus') {
          const status = ctx.webSocketService.getStatus()
          return { id, result: status, type: 'data' }
        } else if (type === 'mutation') {
          if (procedureName === 'start') {
            const result = ctx.webSocketService.start()
            return { id, result, type: 'data' }
          } else if (procedureName === 'stop') {
            ctx.webSocketService.stop()
            return { id, result: true, type: 'data' }
          }
        }
      } else if (moduleName === 'adapters') {
        if (type === 'query') {
          if (procedureName === 'getAll') {
            const adapters = ctx.adapterManager.getAllAdapters()
            // Use our helper to create serializable adapter objects
            const result = adapters.map((adapter) => createSerializableAdapter(adapter))
            return { id, result, type: 'data' }
          } else if (procedureName === 'getById') {
            const adapter = ctx.adapterManager.getAdapter(input)
            if (!adapter) {
              return { id, result: null, type: 'data' }
            }
            // Use our helper for consistent serialization
            const result = createSerializableAdapter(adapter)
            return { id, result, type: 'data' }
          } else if (procedureName === 'getTypes') {
            const types = ctx.adapterManager.getAvailableAdapterTypes()
            return { id, result: types, type: 'data' }
          }
        } else if (type === 'mutation') {
          if (procedureName === 'create') {
            const adapter = await ctx.adapterManager.createAdapter(input)
            // Use our helper for consistent serialization
            const result = createSerializableAdapter(adapter)
            return { id, result, type: 'data' }
          } else if (procedureName === 'start') {
            await ctx.adapterManager.startAdapter(input)
            return { id, result: true, type: 'data' }
          } else if (procedureName === 'stop') {
            await ctx.adapterManager.stopAdapter(input)
            return { id, result: true, type: 'data' }
          } else if (procedureName === 'update') {
            await ctx.adapterManager.updateAdapter(input.id, input.config)
            return { id, result: true, type: 'data' }
          } else if (procedureName === 'delete') {
            await ctx.adapterManager.deleteAdapter(input)
            return { id, result: true, type: 'data' }
          }
        }
      } else if (moduleName === 'auth') {
        if (type === 'query') {
          if (procedureName === 'getStatus') {
            const provider = input as AuthProvider
            const status = ctx.authService.getStatus(provider)
            // Convert status to serializable object
            const result = {
              state: status.state,
              provider: status.provider,
              lastUpdated: status.lastUpdated,
              error: status.error ? status.error.message : undefined,
              expiresAt: status.expiresAt,
              userId: status.userId,
              username: status.username,
              isAuthenticated: status.state === AuthState.AUTHENTICATED
            }
            return { id, result, type: 'data' }
          } else if (procedureName === 'isAuthenticated') {
            const provider = input as AuthProvider
            const result = ctx.authService.isAuthenticated(provider)
            return { id, result, type: 'data' }
          }
        } else if (type === 'mutation') {
          if (procedureName === 'authenticate') {
            const { provider, options } = input as { provider: AuthProvider; options: AuthOptions }
            const result = await ctx.authService.authenticate(provider, options)
            // Return a safe version of the result (without tokens)
            const safeResult = {
              success: result.success,
              error: result.error ? result.error.message : undefined,
              userId: result.userId,
              username: result.username
            }
            return { id, result: safeResult, type: 'data' }
          } else if (procedureName === 'refreshToken') {
            const provider = input as AuthProvider
            const result = await ctx.authService.refreshToken(provider)
            // Return a safe version of the result (without tokens)
            const safeResult = {
              success: result.success,
              error: result.error ? result.error.message : undefined,
              userId: result.userId,
              username: result.username
            }
            return { id, result: safeResult, type: 'data' }
          } else if (procedureName === 'revokeToken') {
            const provider = input as AuthProvider
            await ctx.authService.revokeToken(provider)
            return { id, result: true, type: 'data' }
          }
        } else if (type === 'subscription') {
          if (procedureName === 'onStatusChange') {
            const provider = input as AuthProvider
            const subChannelName = `${TRPC_CHANNEL}:${id}`
            
            // Track this client's ID
            ctx.senderIds.add(event.sender.id)
            
            // Subscribe to auth status changes
            const subscription = ctx.authService.status$(provider).subscribe({
              next: (status) => {
                // Convert status to a safe serializable object
                const safeStatus = {
                  state: status.state,
                  provider: status.provider,
                  lastUpdated: status.lastUpdated,
                  error: status.error ? status.error.message : undefined,
                  expiresAt: status.expiresAt,
                  userId: status.userId,
                  username: status.username,
                  isAuthenticated: status.state === AuthState.AUTHENTICATED
                }
                
                // Send to the client
                event.sender.send(subChannelName, {
                  id,
                  result: safeStatus,
                  type: 'data'
                })
              },
              error: (err) => {
                // Send error to client
                console.error('Error in auth status subscription:', err)
                event.sender.send(subChannelName, {
                  id,
                  error: toSerializableError(err),
                  type: 'error'
                })
              }
            })
            
            // Set up cleanup
            ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
              subscription.unsubscribe()
              console.log(`Cleaned up auth status subscription for ${id}`)
            })
            
            return { id, type: 'started' }
          } else if (procedureName === 'onDeviceCode') {
            // This subscription is for the device code flow
            const subChannelName = `${TRPC_CHANNEL}:${id}`
            
            // Track this client's ID
            ctx.senderIds.add(event.sender.id)
            
            // Subscribe to auth events for device code
            const subscription = ctx.mainEventBus.events$
              .pipe(
                filter((e) => 
                  e.category === EventCategory.AUTH && 
                  (e.type === 'device_code_received' || e.type === 'authentication_failed')
                )
              )
              .subscribe({
                next: (authEvent) => {
                  // Send to the client
                  event.sender.send(subChannelName, {
                    id,
                    result: {
                      type: authEvent.type,
                      ...authEvent.payload
                    },
                    type: 'data'
                  })
                },
                error: (err) => {
                  // Send error to client
                  console.error('Error in device code subscription:', err)
                  event.sender.send(subChannelName, {
                    id,
                    error: toSerializableError(err),
                    type: 'error'
                  })
                }
              })
            
            // Set up cleanup
            ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
              subscription.unsubscribe()
              console.log(`Cleaned up device code subscription for ${id}`)
            })
            
            return { id, type: 'started' }
          }
        }
      }

      throw new Error(`Unhandled request: ${type} ${path}`)
    } catch (error) {
      // Handle errors using our standardized error format
      console.error(`Error handling tRPC request: ${type} ${path}`, error)
      return {
        id,
        error: toSerializableError(error),
        type: 'error'
      }
    }
  })
}
