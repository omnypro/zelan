import { useState, useEffect, useCallback } from 'react';
import { useTrpc } from './useTrpc';

interface AdapterConfig {
  [key: string]: unknown;
}

/**
 * Hook for managing adapter settings
 */
export function useAdapterSettings(adapterId: string) {
  const trpc = useTrpc();
  const [settings, setSettings] = useState<AdapterConfig | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  /**
   * Fetch the current adapter settings
   */
  const fetchSettings = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    if (!adapterId) {
      console.error('No adapter ID provided');
      setError('No adapter ID provided');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const adapterStatus = await trpc.client.adapter.getStatus.query(adapterId);
      
      if (adapterStatus.status === 'not-found') {
        setError(`Adapter ${adapterId} not found`);
        return null;
      }
      
      // Get config from adapter status
      const config = adapterStatus.config || {};
      setSettings(config);
      return config;
    } catch (error) {
      console.error(`Error fetching adapter settings for ${adapterId}:`, error);
      const message = error instanceof Error ? error.message : 'Failed to fetch adapter settings';
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, adapterId]);
  
  /**
   * Update the adapter settings
   */
  const updateSettings = useCallback(async (newSettings: AdapterConfig) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    if (!adapterId) {
      console.error('No adapter ID provided');
      setError('No adapter ID provided');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await trpc.client.adapter.updateConfig.mutate({
        adapterId,
        config: newSettings
      });
      
      if (result.success) {
        // Refresh settings after update
        await fetchSettings();
        return true;
      } else {
        setError(result.error || 'Failed to update adapter settings');
        return false;
      }
    } catch (error) {
      console.error(`Error updating adapter settings for ${adapterId}:`, error);
      const message = error instanceof Error ? error.message : 'Failed to update adapter settings';
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, adapterId, fetchSettings]);
  
  /**
   * Enable or disable the adapter
   */
  const setEnabled = useCallback(async (enabled: boolean) => {
    if (!settings) {
      console.error('No settings available');
      return false;
    }
    
    return await updateSettings({
      ...settings,
      enabled
    });
  }, [settings, updateSettings]);
  
  /**
   * Set auto-connect for the adapter
   */
  const setAutoConnect = useCallback(async (autoConnect: boolean) => {
    if (!settings) {
      console.error('No settings available');
      return false;
    }
    
    return await updateSettings({
      ...settings,
      autoConnect
    });
  }, [settings, updateSettings]);
  
  // Fetch adapter settings on mount and when adapter ID changes
  useEffect(() => {
    if (trpc.client && adapterId) {
      fetchSettings();
    }
  }, [trpc.client, adapterId, fetchSettings]);
  
  return {
    settings,
    isLoading,
    error,
    fetchSettings,
    updateSettings,
    setEnabled,
    setAutoConnect
  };
}