import { useCallback, useEffect, useState } from 'react';
import type { 
  AdapterStatus, 
  OperationResult, 
  WebSocketStatus, 
  WebSocketConfig,
  AuthState, 
  EventsResponse 
} from '../trpc/shared/types';

/**
 * Hook for interacting with the Electron API
 */
export function useElectronAPI() {
  // Check if we're running in Electron context
  const [isElectron, setIsElectron] = useState<boolean>(false);

  useEffect(() => {
    // Check if the Electron API is available
    setIsElectron(typeof window !== 'undefined' && 'zelan' in window);
  }, []);

  // Adapter functions
  const getAdapterStatus = useCallback((adapterId: string): Promise<AdapterStatus> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.adapters.getStatus(adapterId);
  }, [isElectron]);

  const connectAdapter = useCallback((adapterId: string): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.adapters.connect(adapterId);
  }, [isElectron]);

  const disconnectAdapter = useCallback((adapterId: string): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.adapters.disconnect(adapterId);
  }, [isElectron]);

  const updateAdapterConfig = useCallback((adapterId: string, config: Record<string, unknown>): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.adapters.updateConfig(adapterId, config);
  }, [isElectron]);

  // WebSocket server functions
  const getWebSocketStatus = useCallback((): Promise<WebSocketStatus> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.websocket.getStatus();
  }, [isElectron]);

  const startWebSocketServer = useCallback((): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.websocket.start();
  }, [isElectron]);

  const stopWebSocketServer = useCallback((): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.websocket.stop();
  }, [isElectron]);

  const updateWebSocketConfig = useCallback((config: WebSocketConfig): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.websocket.updateConfig(config);
  }, [isElectron]);

  // Auth functions
  const getAuthState = useCallback((serviceId: string): Promise<AuthState> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.auth.getState(serviceId);
  }, [isElectron]);

  const authenticate = useCallback((serviceId: string): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.auth.authenticate(serviceId);
  }, [isElectron]);

  const logout = useCallback((serviceId: string): Promise<OperationResult> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.auth.logout(serviceId);
  }, [isElectron]);

  // Event functions
  const getRecentEvents = useCallback((count: number = 10): Promise<EventsResponse> => {
    if (!isElectron || !window.zelan) return Promise.reject('Not running in Electron');
    return window.zelan.events.getRecentEvents(count);
  }, [isElectron]);

  return {
    isElectron,
    adapters: {
      getStatus: getAdapterStatus,
      connect: connectAdapter,
      disconnect: disconnectAdapter,
      updateConfig: updateAdapterConfig,
    },
    websocket: {
      getStatus: getWebSocketStatus,
      start: startWebSocketServer,
      stop: stopWebSocketServer,
      updateConfig: updateWebSocketConfig,
    },
    auth: {
      getState: getAuthState,
      authenticate,
      logout,
    },
    events: {
      getRecentEvents,
    },
  };
}