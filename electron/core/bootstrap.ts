import { ConfigManager } from './config/configManager';
import { AdapterSettingsManager } from './config/adapterSettingsManager';
import { UserDataManager } from './config/userDataManager';
import { WebSocketServer } from './websocket';
import { TokenManager, AuthService } from './auth';
import { ConfigStore, AdapterSettingsStore, UserDataStore, TokenStore } from '~/store';
import { EventBus, EventType, createEvent, BaseEventSchema, EventCache } from './events';
import { AdapterManager, TestAdapter, ObsAdapter } from './adapters';

/**
 * Initialize and bootstrap the Electron main process components
 */
export async function initializeMainProcess(config: {
  enableTestAdapter?: boolean;
  enableObsAdapter?: boolean;
  startWebSocketServer?: boolean;
  webSocketPort?: number;
  webSocketPath?: string;
  initTrpc?: boolean;
}) {
  console.log('Initializing Electron main process components...');

  // Initialize stores
  const configStore = ConfigStore.getInstance();
  const adapterSettingsStore = AdapterSettingsStore.getInstance();
  const userDataStore = UserDataStore.getInstance();
  const tokenStore = TokenStore.getInstance();
  
  // Initialize managers
  const configManager = ConfigManager.getInstance();
  const adapterSettingsManager = AdapterSettingsManager.getInstance();
  const userDataManager = UserDataManager.getInstance();
  const tokenManager = TokenManager.getInstance();

  console.log('Core electron stores and managers initialized');

  // Get instances of core services
  const eventBus = EventBus.getInstance();
  const adapterManager = AdapterManager.getInstance();
  const authService = AuthService.getInstance();

  // Initialize EventCache for storing recent events
  const eventCache = EventCache.getInstance();
  console.log('Event cache initialized');

  // Publish application startup event
  eventBus.publish(
    createEvent(BaseEventSchema, {
      type: EventType.SYSTEM_STARTUP,
      source: 'system'
    })
  );

  // Add test adapter if enabled
  if (config.enableTestAdapter) {
    console.log('Enabling test adapter');

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
    console.log('Enabling OBS adapter');

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
    const wsPort = config.webSocketPort || configManager.getWebSocketConfig().port;
    const wsPath = config.webSocketPath || configManager.getWebSocketConfig().path;
    
    console.log(`Starting WebSocket server on port ${wsPort}`);
    
    const wsServer = WebSocketServer.getInstance({
      port: wsPort,
      path: wsPath
    });
    
    wsServer.start();
  }

  // Return initialized components
  return {
    stores: {
      configStore,
      adapterSettingsStore,
      userDataStore,
      tokenStore
    },
    managers: {
      configManager,
      adapterSettingsManager,
      userDataManager,
      tokenManager
    },
    services: {
      eventBus,
      adapterManager,
      authService,
      eventCache
    }
  };
}

/**
 * Shutdown Electron main process components
 */
export async function shutdownMainProcess() {
  console.log('Shutting down Electron main process components...');

  // Get instances of core services
  const eventBus = EventBus.getInstance();
  const adapterManager = AdapterManager.getInstance();

  // Publish application shutdown event
  eventBus.publish(
    createEvent(BaseEventSchema, {
      type: EventType.SYSTEM_SHUTDOWN,
      source: 'system'
    })
  );

  // Disconnect all adapters
  await adapterManager.disconnectAll();

  // Clean up services
  adapterManager.destroy();

  // Get WebSocket server instance if it exists
  try {
    const wsServer = WebSocketServer.getInstance();
    if (wsServer.isRunning()) {
      console.log('Stopping WebSocket server...');
      wsServer.stop();
    }
  } catch (error) {
    console.error('Error stopping WebSocket server:', error);
  }

  console.log('Electron main process components shut down successfully');
}