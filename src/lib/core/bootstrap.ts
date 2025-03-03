import { EventBus, EventType, createEvent, BaseEventSchema, EventCache } from './events'
import { AdapterManager } from './adapters'
import { TestAdapter } from './adapters'
import { WebSocketServer } from './websocket'
import { AuthService } from './auth'

/**
 * Bootstrap the application core
 * This initializes all core services in the correct order
 */
export async function bootstrap(config: {
  enableTestAdapter?: boolean
  startWebSocketServer?: boolean
  webSocketPort?: number
  webSocketPath?: string
  initTrpc?: boolean
}) {
  // Get instances of core services
  const eventBus = EventBus.getInstance()
  const adapterManager = AdapterManager.getInstance()
  const authService = AuthService.getInstance()

  // Initialize EventCache for storing recent events
  // This needs to be done early to capture startup events
  const eventCache = EventCache.getInstance()
  console.log('Event cache initialized')

  // Publish application startup event
  eventBus.publish(
    createEvent(BaseEventSchema, {
      type: EventType.SYSTEM_STARTUP,
      source: 'system'
    })
  )

  // Add test adapter if enabled
  if (config.enableTestAdapter) {
    console.log('Enabling test adapter')
    const testAdapter = new TestAdapter({
      interval: 2000,
      generateErrors: false
    })

    adapterManager.registerAdapter(testAdapter)
  }

  // Start WebSocket server if enabled
  if (config.startWebSocketServer) {
    console.log(`Starting WebSocket server on port ${config.webSocketPort || 8080}`)
    const wsServer = WebSocketServer.getInstance({
      port: config.webSocketPort || 8080,
      path: config.webSocketPath || '/events'
    })

    wsServer.start()
  }

  // Return the initialized services
  return {
    eventBus,
    adapterManager,
    authService,
    eventCache
  }
}

/**
 * Shutdown the application core
 * This cleans up all services in the correct order
 */
export async function shutdown() {
  console.log('Shutting down core services')

  // Get instances of core services
  const eventBus = EventBus.getInstance()
  const adapterManager = AdapterManager.getInstance()
  const wsServer = WebSocketServer.getInstance()

  // Publish application shutdown event
  eventBus.publish(
    createEvent(BaseEventSchema, {
      type: EventType.SYSTEM_SHUTDOWN,
      source: 'system'
    })
  )

  // Stop WebSocket server
  wsServer.stop()

  // Disconnect all adapters
  await adapterManager.disconnectAll()

  // Clean up services
  adapterManager.destroy()
  wsServer.destroy()

  console.log('Core services shut down successfully')
}
