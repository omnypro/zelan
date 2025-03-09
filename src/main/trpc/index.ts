import { ipcMain } from 'electron'
import { createContext } from './context'
import { appRouter } from './routers'
import { TRPCError } from '@trpc/server'
import { observable } from '@trpc/server/observable'
import { MainEventBus } from '@m/services/eventBus'
import { AdapterManager, ReconnectionManager } from '@m/services/adapters'
import { ConfigStore } from '@s/core/config'
import { AuthService } from '@m/services/auth'
import { getLoggingService } from '@m/services/logging'

// Create a simple adapter for electron IPC
const createElectronAdapter = (appRouter: typeof appRouter) => {
  // Get the tRPC caller for the router
  const trpc = appRouter.createCaller({} as any)

  // Create a logger for tRPC
  const logger = getLoggingService().createLogger('tRPC')

  // Channel for tRPC requests
  const TRPC_CHANNEL = 'zelan:trpc'

  // Return the handler for IPC
  return async (event: Electron.IpcMainInvokeEvent, opts: any) => {
    const { id, type, path, input } = opts

    logger.debug(`Received tRPC request: ${type} ${path}`, { id, input })

    try {
      // Parse the path
      const [moduleName, procedureName] = path.split('.')

      if (!moduleName || !procedureName) {
        throw new Error(`Invalid path format: ${path}. Expected "module.procedure"`)
      }

      // Get the module
      const module = appRouter._def.procedures[moduleName]
      if (!module) {
        throw new Error(`Module not found: ${moduleName}`)
      }

      // Get the procedure
      const procedure = module._def.procedures[procedureName]
      if (!procedure) {
        throw new Error(`Procedure not found: ${procedureName}`)
      }

      // Handle different types of requests
      if (type === 'query') {
        const result = await trpc[moduleName][procedureName].query(input)
        return { id, result, type: 'data' }
      } else if (type === 'mutation') {
        const result = await trpc[moduleName][procedureName].mutate(input)
        return { id, result, type: 'data' }
      } else if (type === 'subscription') {
        // For subscriptions, we need to create an observable
        const subChannelName = `${TRPC_CHANNEL}:${id}`

        // Set up stop handler
        ipcMain.once(`${subChannelName}:stop`, () => {
          logger.debug(`Subscription stopped: ${moduleName}.${procedureName}`, { id })
          // This will be handled by the client
        })

        // Create the subscription
        const subscription = trpc[moduleName][procedureName].subscribe(input)

        // Set up the subscription
        subscription.subscribe({
          next: (data) => {
            if (!event.sender.isDestroyed()) {
              event.sender.send(subChannelName, { id, data, type: 'data' })
            }
          },
          error: (err) => {
            if (!event.sender.isDestroyed()) {
              event.sender.send(subChannelName, {
                id,
                error: err instanceof Error ? { message: err.message } : { message: String(err) },
                type: 'error'
              })
            }
          },
          complete: () => {
            if (!event.sender.isDestroyed()) {
              event.sender.send(subChannelName, { id, type: 'stopped' })
            }
          }
        })

        // Return initial acknowledgement
        return { id, type: 'started' }
      } else {
        throw new Error(`Unknown request type: ${type}`)
      }
    } catch (error) {
      logger.error(`Error processing request: ${path}`, {
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined
      })

      return {
        id,
        error: error instanceof Error ? { message: error.message } : { message: String(error) },
        type: 'error'
      }
    }
  }
}

/**
 * Setup tRPC server over Electron IPC
 */
export function setupTRPCServer(
  mainEventBus: MainEventBus,
  adapterManager: AdapterManager,
  configStore: ConfigStore,
  authService: AuthService,
  reconnectionManager: ReconnectionManager
) {
  const ctx = createContext(mainEventBus, adapterManager, reconnectionManager, configStore, authService)

  // Channel for tRPC requests
  const TRPC_CHANNEL = 'zelan:trpc'

  // Create a logger for tRPC
  const logger = getLoggingService().createLogger('tRPC')

  logger.info('Setting up tRPC server', { channel: TRPC_CHANNEL })

  // Create the router with context
  const router = appRouter.createCaller(ctx)

  // Register the IPC handler
  ipcMain.handle(TRPC_CHANNEL, createElectronAdapter(appRouter))
}