import { useState, useEffect, useCallback } from 'react';
import { useTrpc } from './useTrpc';
import type { AdapterStatus } from '../trpc/shared/types';

/**
 * Hook for interacting with adapters through tRPC
 * Provides adapter status and control functions
 */
export function useAdapter(adapterId: string) {
  const trpc = useTrpc();
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
      setStatus(adapterStatus);
    } catch (err) {
      console.error('Error fetching adapter status:', err);
      const message = err instanceof Error ? err.message : 'Failed to fetch adapter status';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client, adapterId]);

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
        await trpc.client.adapter.updateConfig.mutate({
          adapterId,
          config,
        });
        await fetchStatus(); // Refresh status after updating config
      } catch (err) {
        console.error('Error updating adapter config:', err);
        const message = err instanceof Error ? err.message : 'Failed to update adapter config';
        setError(message);
      } finally {
        setIsLoading(false);
      }
    },
    [trpc.client, adapterId, fetchStatus]
  );

  return {
    status,
    isLoading,
    error,
    connect,
    disconnect,
    updateConfig,
    refreshStatus: fetchStatus,
  };
}