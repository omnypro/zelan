import { useCallback } from 'react';
import { AdapterManager, AdapterState, ServiceAdapter } from '../core/adapters';
import { useObservable } from './useObservable';

/**
 * Hook for interacting with adapters
 * Provides adapter state and control functions
 */
export function useAdapter<T extends ServiceAdapter>(adapterId: string) {
  const adapterManager = AdapterManager.getInstance();
  
  // Get the adapter
  const adapter = adapterManager.getAdapter<T>(adapterId);
  
  // Get adapter state if available
  const adapterState = adapter ? 
    useObservable(adapter.state$, adapter.state) : 
    AdapterState.DISCONNECTED;
  
  // Convenience state getters
  const isConnected = adapterState === AdapterState.CONNECTED;
  const isConnecting = adapterState === AdapterState.CONNECTING;
  const hasError = adapterState === AdapterState.ERROR;
  
  // Connect the adapter
  const connect = useCallback(async () => {
    if (adapter) {
      await adapter.connect();
    } else {
      throw new Error(`Adapter with ID ${adapterId} not found`);
    }
  }, [adapter, adapterId]);
  
  // Disconnect the adapter
  const disconnect = useCallback(async () => {
    if (adapter) {
      await adapter.disconnect();
    } else {
      throw new Error(`Adapter with ID ${adapterId} not found`);
    }
  }, [adapter, adapterId]);
  
  // Update adapter configuration
  const updateConfig = useCallback((config: any) => {
    if (adapter) {
      adapter.updateConfig(config);
    } else {
      throw new Error(`Adapter with ID ${adapterId} not found`);
    }
  }, [adapter, adapterId]);
  
  return {
    adapter,
    adapterState,
    isConnected,
    isConnecting,
    hasError,
    connect,
    disconnect,
    updateConfig,
    config: adapter ? adapter.config : null,
  };
}