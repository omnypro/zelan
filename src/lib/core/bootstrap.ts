import { EventBus, EventType, createEvent, BaseEventSchema } from './events';
import { AdapterManager } from './adapters';
import { TestAdapter } from './adapters';
import { WebSocketServer } from './websocket';
import { AuthService } from './auth';

/**
 * Bootstrap the application core
 * This initializes all core services in the correct order
 */
export async function bootstrap(config: {
  enableTestAdapter?: boolean;
  startWebSocketServer?: boolean;
  webSocketPort?: number;
}) {
  // Get instances of core services
  const eventBus = EventBus.getInstance();
  const adapterManager = AdapterManager.getInstance();
  const authService = AuthService.getInstance();
  
  // Publish application startup event
  eventBus.publish(createEvent(
    BaseEventSchema,
    {
      type: EventType.SYSTEM_STARTUP,
      source: 'system',
    }
  ));
  
  // Add test adapter if enabled
  if (config.enableTestAdapter) {
    const testAdapter = new TestAdapter({
      interval: 2000,
      generateErrors: false,
    });
    
    adapterManager.registerAdapter(testAdapter);
  }
  
  // Start WebSocket server if enabled
  if (config.startWebSocketServer) {
    const wsServer = WebSocketServer.getInstance({
      port: config.webSocketPort || 8080,
    });
    
    wsServer.start();
  }
  
  // Return the initialized services
  return {
    eventBus,
    adapterManager,
    authService,
  };
}

/**
 * Shutdown the application core
 * This cleans up all services in the correct order
 */
export async function shutdown() {
  // Get instances of core services
  const eventBus = EventBus.getInstance();
  const adapterManager = AdapterManager.getInstance();
  const wsServer = WebSocketServer.getInstance();
  
  // Publish application shutdown event
  eventBus.publish(createEvent(
    BaseEventSchema,
    {
      type: EventType.SYSTEM_SHUTDOWN,
      source: 'system',
    }
  ));
  
  // Stop WebSocket server
  wsServer.stop();
  
  // Disconnect all adapters
  await adapterManager.disconnectAll();
  
  // Clean up services
  adapterManager.destroy();
  wsServer.destroy();
}