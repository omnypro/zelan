import { useCallback } from 'react';
import { WebSocketServer, WebSocketServerConfig } from '../core/websocket';
import { useObservable } from './useObservable';

/**
 * Hook for interacting with the WebSocket server
 * Provides server state and control functions
 */
export function useWebSocketServer() {
  const wsServer = WebSocketServer.getInstance();
  
  // Get server state as an observable
  const isRunning = useObservable(wsServer.state$(), wsServer.isRunning());
  
  // Start the server
  const start = useCallback(() => {
    wsServer.start();
  }, []);
  
  // Stop the server
  const stop = useCallback(() => {
    wsServer.stop();
  }, []);
  
  // Update server configuration
  const updateConfig = useCallback((config: Partial<WebSocketServerConfig>) => {
    wsServer.updateConfig(config);
  }, []);
  
  // Get client count
  const getClientCount = useCallback(() => {
    return wsServer.getClientCount();
  }, []);
  
  // Broadcast a message to all clients
  const broadcast = useCallback((type: string, payload: any) => {
    wsServer.broadcast({
      type,
      payload,
      timestamp: Date.now(),
    });
  }, []);
  
  return {
    isRunning,
    start,
    stop,
    updateConfig,
    getClientCount,
    broadcast,
  };
}