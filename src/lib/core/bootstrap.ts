import { EventBus, EventType, createEvent, BaseEventSchema, EventCache } from './events'
import { AdapterManager } from './adapters'
import { AuthService } from './auth'
import { client } from '../../trpc/client'

/**
 * Bootstrap the renderer process components
 * Note: Main process bootstrap is now in electron/core/bootstrap.ts
 */
export async function bootstrapRenderer() {
  console.log('Initializing renderer process components...')

  // Get instances of core services
  const eventBus = EventBus.getInstance()
  const adapterManager = AdapterManager.getInstance()
  const authService = AuthService.getInstance()
  const eventCache = EventCache.getInstance()
  
  console.log('Renderer core services initialized')

  // Initialize tRPC client connection
  await client.config.getAppConfig.query()
  console.log('tRPC client connection established')

  // Return the initialized services
  return {
    eventBus,
    adapterManager,
    authService,
    eventCache
  }
}

/**
 * Shutdown the renderer process
 */
export async function shutdownRenderer() {
  console.log('Shutting down renderer process components...')

  // Get instances of core services
  const eventBus = EventBus.getInstance()
  const adapterManager = AdapterManager.getInstance()

  // Publish application shutdown event
  eventBus.publish(
    createEvent(BaseEventSchema, {
      type: EventType.SYSTEM_SHUTDOWN,
      source: 'system'
    })
  )

  // Clean up services
  adapterManager.destroy()

  console.log('Renderer process components shut down successfully')
}
