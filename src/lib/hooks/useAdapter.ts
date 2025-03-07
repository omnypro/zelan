import { useState, useEffect, useCallback } from 'react';
import { useTrpc } from './useTrpc';
import { useAdapterSettings } from './useAdapterSettings';
import type { AdapterStatus } from '@/lib/trpc/shared/types';
import { BehaviorSubject } from 'rxjs';

// Create a subject for each adapter's status
const adapterStatusSubjects = new Map<string, BehaviorSubject<AdapterStatus | null>>();

// Get or create a subject for an adapter
function getStatusSubject(adapterId: string): BehaviorSubject<AdapterStatus | null> {
  if (!adapterStatusSubjects.has(adapterId)) {
    adapterStatusSubjects.set(adapterId, new BehaviorSubject<AdapterStatus | null>(null));
  }
  return adapterStatusSubjects.get(adapterId)!;
}

/**
 * Hook for interacting with adapters through tRPC
 * Provides adapter status and control functions
 */
export function useAdapter(adapterId: string) {
  const trpc = useTrpc();
  const adapterSettings = useAdapterSettings(adapterId);
  const [status, setStatus] = useState<AdapterStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch adapter status
  const fetchStatus = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const adapterStatus = await trpc.client.adapter.getStatus.query(adapterId);
      
      // Update the behavior subject
      const subject = getStatusSubject(adapterId);
      subject.next(adapterStatus);
      
      setStatus(adapterStatus);
      
      // Also refresh settings to ensure they're in sync
      await adapterSettings.fetchSettings();
    } catch (err) {
      console.error('Error fetching adapter status:', err);
      const message = err instanceof Error ? err.message : 'Failed to fetch adapter status';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, adapterId, adapterSettings]);

  // Initial load
  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  // Connect the adapter
  const connect = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await trpc.client.adapter.connect.mutate(adapterId);
      await fetchStatus(); // Refresh status after connecting
    } catch (err) {
      console.error('Error connecting adapter:', err);
      const message = err instanceof Error ? err.message : 'Failed to connect adapter';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, adapterId, fetchStatus]);

  // Disconnect the adapter
  const disconnect = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await trpc.client.adapter.disconnect.mutate(adapterId);
      await fetchStatus(); // Refresh status after disconnecting
    } catch (err) {
      console.error('Error disconnecting adapter:', err);
      const message = err instanceof Error ? err.message : 'Failed to disconnect adapter';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, adapterId, fetchStatus]);

  // Update adapter configuration
  const updateConfig = useCallback(
    async (config: Record<string, unknown>) => {
      if (!trpc.client) {
        console.error('TRPC client not available');
        setError('TRPC client not available');
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        // First update persistent settings
        await adapterSettings.updateSettings(config);
        
        // Then update runtime config
        await trpc.client.adapter.updateConfig.mutate({
          adapterId,
          config,
        });
        
        // Refresh status after updating config
        await fetchStatus();
      } catch (err) {
        console.error('Error updating adapter config:', err);
        const message = err instanceof Error ? err.message : 'Failed to update adapter config';
        setError(message);
      } finally {
        setIsLoading(false);
      }
    },
    [trpc.client, adapterId, fetchStatus, adapterSettings]
  );
  
  // Subscribe to status changes
  useEffect(() => {
    const subject = getStatusSubject(adapterId);
    const subscription = subject.subscribe(newStatus => {
      if (newStatus) {
        setStatus(newStatus);
      }
    });
    
    return () => {
      subscription.unsubscribe();
    };
  }, [adapterId]);

  return {
    status,
    isLoading,
    error,
    settings: adapterSettings.settings,
    connect,
    disconnect,
    updateConfig,
    refreshStatus: fetchStatus,
    
    // Also expose settings functions directly
    updateSettings: adapterSettings.updateSettings,
    setEnabled: adapterSettings.setEnabled,
    setAutoConnect: adapterSettings.setAutoConnect
  };
}