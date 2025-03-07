import { EventBus, EventType, createEvent, BaseEventSchema, EventCache } from './events'
import { AdapterManager } from './adapters'
import { TestAdapter, ObsAdapter } from './adapters'
import { WebSocketServer } from './websocket'
import { AuthService } from './auth'
import { ConfigStore, AdapterSettingsStore, UserDataStore } from '../../../electron/store'

/**
 * Bootstrap the application core
 * This initializes all core services in the correct order
 */
export async function bootstrap(config: {
  enableTestAdapter?: boolean
  enableObsAdapter?: boolean
  startWebSocketServer?: boolean
  webSocketPort?: number
  webSocketPath?: string
  initTrpc?: boolean
}) {
  // Initialize configuration and persistence layer first
  const configStore = ConfigStore.getInstance()
  const adapterSettingsStore = AdapterSettingsStore.getInstance()
  const userDataStore = UserDataStore.getInstance()
  console.log('Configuration and persistence layer initialized')
  
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
    
    let testConfig = {
      interval: 2000,
      generateErrors: false
    };
    
    // Try to load settings
    try {
      const savedSettings = adapterSettingsStore.getSettings('test-adapter');
      if (savedSettings) {
        console.log('Using saved test adapter settings');
        testConfig = {
          ...testConfig,
          ...savedSettings,
        };
      } else {
        console.log('Using default test adapter settings');
      }
    } catch (error) {
      console.warn('Could not load test adapter settings, using defaults');
    }
    
    const testAdapter = new TestAdapter(testConfig);
    adapterManager.registerAdapter(testAdapter);
  }
  
  // Add OBS adapter if enabled
  if (config.enableObsAdapter) {
    console.log('Enabling OBS adapter')
    
    // Try to get saved settings first
    let obsConfig = {
      host: 'localhost',
      port: 4455,
      autoConnect: false, // Don't auto-connect until configured
      enabled: true       // Adapter is enabled but won't connect automatically
    };
    
    // Try to load settings from settings store
    try {
      const savedSettings = adapterSettingsStore.getSettings('obs-adapter');
      if (savedSettings) {
        console.log('Using saved OBS adapter settings');
        // Keep some defaults to ensure adapter works
        obsConfig = {
          ...obsConfig,
          ...savedSettings,
        };
      }
    } catch (error) {
      console.warn('Could not load OBS adapter settings, using defaults');
    }
    
    const obsAdapter = new ObsAdapter(obsConfig);
    adapterManager.registerAdapter(obsAdapter);
    
    // Log connection parameters for debugging
    console.log('OBS adapter initial config:', obsAdapter.config);
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
    // Configuration
    configStore,
    adapterSettingsStore,
    userDataStore,
    
    // Core services
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
