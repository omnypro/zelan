import { ConfigManager } from './config/configManager';
import { AdapterSettingsManager } from './config/adapterSettingsManager';
import { UserDataManager } from './config/userDataManager';
import { WebSocketServer } from './websocket/websocketServer';
import { TokenManager } from './auth/tokenManager';
import { ConfigStore, AdapterSettingsStore, UserDataStore, TokenStore } from '../store';

/**
 * Initialize and bootstrap the Electron main process components
 */
export async function initializeMainProcess(config: {
  startWebSocketServer?: boolean;
  webSocketPort?: number;
  webSocketPath?: string;
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
    }
  };
}

/**
 * Shutdown Electron main process components
 */
export async function shutdownMainProcess() {
  console.log('Shutting down Electron main process components...');

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