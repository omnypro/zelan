import { useState, useEffect, useCallback } from 'react';
import { useTrpc } from './useTrpc';
import type { WebSocketConfig } from '@/lib/trpc/shared/types';
import { BehaviorSubject } from 'rxjs';

// Reactive subjects for config changes
const appConfigSubject = new BehaviorSubject<Record<string, unknown>>({});
const adapterSettingsSubject = new BehaviorSubject<Record<string, unknown>>({});
const userDataSubject = new BehaviorSubject<Record<string, unknown>>({});

/**
 * Hook for accessing application configuration
 */
export function useConfig() {
  const trpc = useTrpc();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [webSocketConfig, setWebSocketConfig] = useState<WebSocketConfig | null>(null);
  const [appConfig, setAppConfig] = useState<Record<string, unknown>>({});
  const [userData, setUserData] = useState<Record<string, unknown>>({});

  // App Config Operations
  const fetchAppConfig = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const response = await trpc.client.config.getAppConfig.query();
      
      if (response.success) {
        // Update the behavior subject
        appConfigSubject.next(response.data);
        return response.data;
      } else {
        setError(response.error || 'Failed to fetch app configuration');
        return null;
      }
    } catch (error) {
      console.error('Error fetching app config:', error);
      const message = error instanceof Error ? error.message : 'Failed to fetch app configuration';
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  const updateAppConfig = useCallback(async (config: Record<string, unknown>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.config.updateAppConfig.mutate(config);
      
      if (result.success) {
        // Update the behavior subject with merged data
        const currentConfig = appConfigSubject.getValue();
        appConfigSubject.next({ ...currentConfig, ...config });
        return true;
      } else {
        setError(result.error || 'Failed to update app configuration');
        return false;
      }
    } catch (error) {
      console.error('Error updating app config:', error);
      const message = error instanceof Error ? error.message : 'Failed to update app configuration';
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);

  // User Data Operations
  const fetchUserData = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const response = await trpc.client.config.getUserData.query();
      
      if (response.success) {
        // Update the behavior subject
        userDataSubject.next(response.data);
        return response.data;
      } else {
        setError(response.error || 'Failed to fetch user data');
        return null;
      }
    } catch (error) {
      console.error('Error fetching user data:', error);
      const message = error instanceof Error ? error.message : 'Failed to fetch user data';
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  const updateUserData = useCallback(async (data: Record<string, unknown>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.config.updateUserData.mutate(data);
      
      if (result.success) {
        // Update the behavior subject with merged data
        const currentData = userDataSubject.getValue();
        userDataSubject.next({ ...currentData, ...data });
        return true;
      } else {
        setError(result.error || 'Failed to update user data');
        return false;
      }
    } catch (error) {
      console.error('Error updating user data:', error);
      const message = error instanceof Error ? error.message : 'Failed to update user data';
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);

  // WebSocket Config Operations
  const fetchWebSocketConfig = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      // Use the app config to get WebSocket settings
      const appConfig = await fetchAppConfig();
      if (appConfig && appConfig.webSocket) {
        return appConfig.webSocket as WebSocketConfig;
      }
      
      // Fallback to direct status check
      const status = await trpc.client.websocket.getStatus.query();
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
  }, [trpc.client, fetchAppConfig]);
  
  const updateWebSocketConfig = useCallback(async (config: Partial<WebSocketConfig>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      // First update the WebSocket server directly
      const wsResult = await trpc.client.websocket.updateConfig.mutate({
        ...webSocketConfig,
        ...config
      } as WebSocketConfig);
      
      if (wsResult.success) {
        // Then save the config to app settings
        const appConfig = await fetchAppConfig();
        if (appConfig) {
          const updatedConfig = {
            ...appConfig,
            webSocket: {
              ...(appConfig.webSocket as object || {}),
              ...config
            }
          };
          
          await updateAppConfig(updatedConfig);
        }
        
        // Refresh the configuration after update
        const newConfig = await fetchWebSocketConfig();
        if (newConfig) {
          setWebSocketConfig(newConfig);
        }
        return true;
      } else {
        setError(wsResult.error || 'Failed to update WebSocket configuration');
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
  }, [trpc.client, webSocketConfig, fetchWebSocketConfig, fetchAppConfig, updateAppConfig]);
  
  // Adapter Settings Operations
  const getAdapterSettings = useCallback(async (adapterId: string) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const response = await trpc.client.config.getAdapterSettings.query(adapterId);
      
      if (response.success) {
        return response.data;
      } else {
        setError(response.error || `Failed to fetch settings for adapter: ${adapterId}`);
        return null;
      }
    } catch (error) {
      console.error(`Error fetching settings for adapter ${adapterId}:`, error);
      const message = error instanceof Error ? error.message : `Failed to fetch settings for adapter: ${adapterId}`;
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  const getAllAdapterSettings = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const response = await trpc.client.config.getAllAdapterSettings.query();
      
      if (response.success) {
        // Update the behavior subject
        adapterSettingsSubject.next(response.data);
        return response.data;
      } else {
        setError(response.error || 'Failed to fetch adapter settings');
        return null;
      }
    } catch (error) {
      console.error('Error fetching all adapter settings:', error);
      const message = error instanceof Error ? error.message : 'Failed to fetch adapter settings';
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  const updateAdapterSettings = useCallback(async (adapterId: string, settings: Record<string, unknown>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.config.updateAdapterSettings.mutate({
        adapterId,
        settings
      });
      
      if (result.success) {
        // Update the behavior subject with new settings
        const currentSettings = adapterSettingsSubject.getValue();
        adapterSettingsSubject.next({
          ...currentSettings,
          [adapterId]: {
            ...(currentSettings[adapterId] as object || {}),
            ...settings
          }
        });
        return true;
      } else {
        setError(result.error || `Failed to update settings for adapter: ${adapterId}`);
        return false;
      }
    } catch (error) {
      console.error(`Error updating settings for adapter ${adapterId}:`, error);
      const message = error instanceof Error ? error.message : `Failed to update settings for adapter: ${adapterId}`;
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  const setAdapterEnabled = useCallback(async (adapterId: string, enabled: boolean) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.config.setAdapterEnabled.mutate({
        adapterId,
        enabled
      });
      
      if (result.success) {
        // Update the behavior subject
        const currentSettings = adapterSettingsSubject.getValue();
        if (currentSettings[adapterId]) {
          adapterSettingsSubject.next({
            ...currentSettings,
            [adapterId]: {
              ...(currentSettings[adapterId] as object),
              enabled
            }
          });
        }
        return true;
      } else {
        setError(result.error || `Failed to ${enabled ? 'enable' : 'disable'} adapter: ${adapterId}`);
        return false;
      }
    } catch (error) {
      console.error(`Error setting adapter ${adapterId} enabled state:`, error);
      const message = error instanceof Error ? error.message : `Failed to ${enabled ? 'enable' : 'disable'} adapter: ${adapterId}`;
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  const setAdapterAutoConnect = useCallback(async (adapterId: string, autoConnect: boolean) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.config.setAdapterAutoConnect.mutate({
        adapterId,
        autoConnect
      });
      
      if (result.success) {
        // Update the behavior subject
        const currentSettings = adapterSettingsSubject.getValue();
        if (currentSettings[adapterId]) {
          adapterSettingsSubject.next({
            ...currentSettings,
            [adapterId]: {
              ...(currentSettings[adapterId] as object),
              autoConnect
            }
          });
        }
        return true;
      } else {
        setError(result.error || `Failed to set auto-connect for adapter: ${adapterId}`);
        return false;
      }
    } catch (error) {
      console.error(`Error setting adapter ${adapterId} auto-connect:`, error);
      const message = error instanceof Error ? error.message : `Failed to set auto-connect for adapter: ${adapterId}`;
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  // Initialize config on mount - but only once to avoid infinite loops
  useEffect(() => {
    // Create a "mounted" flag to prevent multiple fetches
    let mounted = true;
    
    if (trpc.client && mounted) {
      // Set loading state
      setIsLoading(true);
      
      // Fetch all configuration data in parallel
      Promise.all([
        fetchAppConfig(),
        fetchUserData(),
        getAllAdapterSettings(),
        fetchWebSocketConfig()
      ]).then(([appConfigData, userDataData, _, webSocketConfigData]) => {
        // Only update state if component is still mounted
        if (mounted) {
          if (appConfigData) {
            setAppConfig(appConfigData);
          }
          
          if (userDataData) {
            setUserData(userDataData);
          }
          
          if (webSocketConfigData) {
            setWebSocketConfig(webSocketConfigData);
          }
          
          // Clear loading state
          setIsLoading(false);
        }
      }).catch(error => {
        if (mounted) {
          console.error('Error initializing config:', error);
          setError(error instanceof Error ? error.message : String(error));
          setIsLoading(false);
        }
      });
    }
    
    // Cleanup function to set mounted to false when component unmounts
    return () => {
      mounted = false;
    };
  }, [trpc.client]); // Remove dependencies that could cause re-fetches
  
  // Subscribe to config changes
  useEffect(() => {
    const appConfigSubscription = appConfigSubject.subscribe(config => {
      setAppConfig(config);
    });
    
    const userDataSubscription = userDataSubject.subscribe(data => {
      setUserData(data);
    });
    
    // Clean up subscriptions
    return () => {
      appConfigSubscription.unsubscribe();
      userDataSubscription.unsubscribe();
    };
  }, []);
  
  return {
    // App config
    appConfig,
    fetchAppConfig,
    updateAppConfig,
    
    // User data
    userData,
    fetchUserData,
    updateUserData,
    
    // WebSocket config
    webSocketConfig,
    fetchWebSocketConfig,
    updateWebSocketConfig,
    
    // Adapter settings
    getAdapterSettings,
    getAllAdapterSettings,
    updateAdapterSettings,
    setAdapterEnabled,
    setAdapterAutoConnect,
    
    // State
    isLoading,
    error
  };
}