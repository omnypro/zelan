import { useState, useEffect, useCallback } from 'react';
import { useTrpc } from './useTrpc';
import type { WebSocketConfig } from '../trpc/shared/types';

/**
 * Hook for accessing application configuration
 */
export function useConfig() {
  const trpc = useTrpc();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [webSocketConfig, setWebSocketConfig] = useState<WebSocketConfig | null>(null);
  
  /**
   * Fetch the current WebSocket server configuration
   */
  const fetchWebSocketConfig = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const status = await trpc.client.websocket.getStatus.query();
      // We'll need to extend the tRPC router to add a getConfig procedure
      // For now, we're just using a workaround
      return {
        port: 9090, // Default port
        pingInterval: 30000,
        path: '/events'
      };
    } catch (error) {
      console.error('Error fetching WebSocket config:', error);
      const message = error instanceof Error ? error.message : 'Failed to fetch WebSocket configuration';
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  /**
   * Update the WebSocket server configuration
   */
  const updateWebSocketConfig = useCallback(async (config: Partial<WebSocketConfig>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.websocket.updateConfig.mutate({
        ...webSocketConfig,
        ...config
      } as WebSocketConfig);
      
      if (result.success) {
        // Refresh the configuration after update
        const newConfig = await fetchWebSocketConfig();
        if (newConfig) {
          setWebSocketConfig(newConfig);
        }
        return true;
      } else {
        setError(result.error || 'Failed to update WebSocket configuration');
        return false;
      }
    } catch (error) {
      console.error('Error updating WebSocket config:', error);
      const message = error instanceof Error ? error.message : 'Failed to update WebSocket configuration';
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, webSocketConfig, fetchWebSocketConfig]);
  
  // Fetch WebSocket configuration on mount
  useEffect(() => {
    if (trpc.client) {
      fetchWebSocketConfig().then(config => {
        if (config) {
          setWebSocketConfig(config);
        }
      });
    }
  }, [trpc.client, fetchWebSocketConfig]);
  
  return {
    webSocketConfig,
    updateWebSocketConfig,
    isLoading,
    error,
    fetchWebSocketConfig
  };
}