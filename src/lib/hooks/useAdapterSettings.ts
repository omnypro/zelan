import { useState, useEffect, useCallback, useRef } from 'react';
import { useTrpc } from './useTrpc';

// Settings type
type AdapterSettings = Record<string, unknown>;

/**
 * Hook for working with adapter settings
 * Uses tRPC to communicate with the main process
 */
export function useAdapterSettings(adapterId: string) {
  const { client } = useTrpc();
  const [settings, setSettings] = useState<AdapterSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const isMounted = useRef(true);

  // Fetch settings from backend
  const fetchSettings = useCallback(async () => {
    if (!client) {
      setIsLoading(false);
      setError('tRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const response = await client.config.getAdapterSettings.query(adapterId);
      
      if (isMounted.current) {
        if (response.success) {
          setSettings(response.data as AdapterSettings);
          return response.data;
        } else {
          setError(response.error || 'Failed to fetch settings');
          return null;
        }
      }
    } catch (error) {
      if (isMounted.current) {
        console.error(`Error fetching settings for ${adapterId}:`, error);
        setError(error instanceof Error ? error.message : 'Failed to fetch settings');
      }
      return null;
    } finally {
      if (isMounted.current) {
        setIsLoading(false);
      }
    }
  }, [client, adapterId]);

  // Update settings
  const updateSettings = useCallback(async (newSettings: Partial<AdapterSettings>) => {
    if (!client) return false;
    
    // Optimistic update
    if (settings) {
      setSettings({ ...settings, ...newSettings });
    }
    
    try {
      const result = await client.config.updateAdapterSettings.mutate({
        adapterId,
        settings: newSettings
      });
      
      // If update failed, refresh settings
      if (!result.success) {
        fetchSettings();
      }
      
      return result.success;
    } catch (error) {
      console.error(`Error updating settings for ${adapterId}:`, error);
      
      // Revert on error by fetching fresh data
      fetchSettings();
      return false;
    }
  }, [client, adapterId, settings, fetchSettings]);

  // Convenience methods
  const setEnabled = useCallback((enabled: boolean) => {
    if (!client) return Promise.resolve(false);
    return client.config.setAdapterEnabled.mutate({
      adapterId,
      enabled
    }).then(result => result.success);
  }, [client, adapterId]);

  const setAutoConnect = useCallback((autoConnect: boolean) => {
    if (!client) return Promise.resolve(false);
    return client.config.setAdapterAutoConnect.mutate({
      adapterId,
      autoConnect
    }).then(result => result.success);
  }, [client, adapterId]);

  // Fetch on mount
  useEffect(() => {
    isMounted.current = true;
    
    if (client) {
      fetchSettings();
    }
    
    return () => {
      isMounted.current = false;
    };
  }, [adapterId, client, fetchSettings]);

  return {
    settings,
    isLoading,
    error,
    updateSettings,
    setEnabled,
    setAutoConnect,
    fetchSettings
  };
}