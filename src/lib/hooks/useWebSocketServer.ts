import { useCallback } from 'react';
import { WebSocketConfig } from '../core/websocket';
import { useTrpc } from './useTrpc';

/**
 * Hook for interacting with the WebSocket server via tRPC
 * Provides server state and control functions
 */
export function useWebSocketServer() {
  const { client } = useTrpc();
  
  // Get server status via tRPC
  const getStatus = useCallback(async () => {
    if (!client) return { isRunning: false, clientCount: 0 };
    return await client.websocket.getStatus.query();
  }, [client]);
  
  // Start the server
  const start = useCallback(async () => {
    if (!client) return { success: false, error: "tRPC client not available" };
    return await client.websocket.start.mutate();
  }, [client]);
  
  // Stop the server
  const stop = useCallback(async () => {
    if (!client) return { success: false, error: "tRPC client not available" };
    return await client.websocket.stop.mutate();
  }, [client]);
  
  // Update server configuration
  const updateConfig = useCallback(async (config: Partial<WebSocketConfig>) => {
    if (!client) return { success: false, error: "tRPC client not available" };
    return await client.websocket.updateConfig.mutate(config);
  }, [client]);
  
  return {
    getStatus,
    start,
    stop,
    updateConfig
  };
}