import { useCallback, useEffect, useState } from 'react';
import type { 
  AdapterStatus, 
  OperationResult, 
  WebSocketStatus, 
  WebSocketConfig,
  AuthState, 
  EventsResponse 
} from '../trpc/shared/types';

// Type definitions for configuration functions
type ConfigResponse = {
  success: boolean;
  data?: Record<string, unknown>;
  error?: string;
};

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

  // Configuration functions
  const getAdapterSettings = useCallback((adapterId: string): Promise<ConfigResponse> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.getAdapterSettings(adapterId);
  }, [isElectron]);

  const updateAdapterSettings = useCallback((adapterId: string, settings: Record<string, unknown>): Promise<OperationResult> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.updateAdapterSettings(adapterId, settings);
  }, [isElectron]);

  const getAllAdapterSettings = useCallback((): Promise<ConfigResponse> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.getAllAdapterSettings();
  }, [isElectron]);

  const setAdapterEnabled = useCallback((adapterId: string, enabled: boolean): Promise<OperationResult> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.setAdapterEnabled(adapterId, enabled);
  }, [isElectron]);

  const setAdapterAutoConnect = useCallback((adapterId: string, autoConnect: boolean): Promise<OperationResult> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.setAdapterAutoConnect(adapterId, autoConnect);
  }, [isElectron]);

  const getAppConfig = useCallback((): Promise<ConfigResponse> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.getAppConfig();
  }, [isElectron]);

  const updateAppConfig = useCallback((config: Record<string, unknown>): Promise<OperationResult> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.updateAppConfig(config);
  }, [isElectron]);

  const getUserData = useCallback((): Promise<ConfigResponse> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.getUserData();
  }, [isElectron]);

  const updateUserData = useCallback((data: Record<string, unknown>): Promise<OperationResult> => {
    if (!isElectron || !window.zelan?.config) return Promise.reject('Config API not available');
    return window.zelan.config.updateUserData(data);
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
    config: {
      getAdapterSettings,
      updateAdapterSettings,
      getAllAdapterSettings,
      setAdapterEnabled,
      setAdapterAutoConnect,
      getAppConfig,
      updateAppConfig,
      getUserData,
      updateUserData,
    },
  };
}