import { useState, useEffect } from 'react';
import { AppRouter } from '../../../shared/trpc';
import { inferRouterOutputs, inferRouterInputs } from '@trpc/server';
import { useObservable } from './useObservable';
import { Observable } from 'rxjs';

// Get inferred types from the AppRouter
type RouterOutput = inferRouterOutputs<AppRouter>;
type RouterInput = inferRouterInputs<AppRouter>;

/**
 * Hook to use config via tRPC
 */
export function useTrpcConfig<T>(
  key: string,
  defaultValue: T
): [T, (value: T) => Promise<void>] {
  const [value, setValue] = useState<T>(defaultValue);

  useEffect(() => {
    // Check if trpc client is available with all needed properties
    if (!window.trpc || !window.trpc.config || !window.trpc.config.get) {
      console.error('tRPC client or required properties not available', {
        trpc: !!window.trpc,
        config: window.trpc ? !!window.trpc.config : false,
        get: window.trpc && window.trpc.config ? !!window.trpc.config.get : false
      });
      return;
    }
    
    // Try to get the initial value
    try {
      window.trpc.config.get.query({ key, defaultValue })
        .then(setValue)
        .catch(err => console.error(`Failed to get initial value for ${key}:`, err));
    } catch (err) {
      console.error(`Error invoking config.get.query for ${key}:`, err);
    }

    // Subscribe to changes for this path
    try {
      // Check that onPathChange subscription is available
      if (!window.trpc.config.onPathChange) {
        console.error('tRPC config.onPathChange not available');
        return () => {};
      }
      
      const subscription = window.trpc.config.onPathChange.subscribe(key);
      
      if (!subscription || typeof subscription.subscribe !== 'function') {
        console.error('Invalid subscription object returned');
        return () => {};
      }
      
      const unsubscribe = subscription.subscribe({
        next: (change) => {
          if (change.key === key) {
            setValue(change.value as T);
          } else if (change.key.startsWith(`${key}.`)) {
            // For nested changes, get the full updated value
            try {
              window.trpc.config.get.query({ key, defaultValue }).then(setValue);
            } catch (err) {
              console.error(`Error getting updated value for ${key}:`, err);
            }
          }
        },
        error: (err) => console.error(`Error in config subscription for ${key}:`, err)
      });

      return () => {
        try {
          unsubscribe.unsubscribe();
        } catch (err) {
          console.error('Error unsubscribing:', err);
        }
      };
    } catch (err) {
      console.error('Failed to subscribe to config changes:', err);
      return () => {};
    }
  }, [key, defaultValue]);

  // Function to update the value
  const updateValue = async (newValue: T) => {
    if (!window.trpc || !window.trpc.config || !window.trpc.config.set) {
      console.error('tRPC client or required properties not available for updating');
      return;
    }
    
    try {
      await window.trpc.config.set.mutate({ key, value: newValue });
    } catch (err) {
      console.error(`Failed to update value for ${key}:`, err);
    }
  };

  return [value, updateValue];
}

/**
 * Hook to get the full config via tRPC
 */
export function useTrpcFullConfig() {
  const [config, setConfig] = useState<RouterOutput['config']['getAll']>({} as any);

  useEffect(() => {
    if (!window.trpc || !window.trpc.config || !window.trpc.config.getAll) {
      console.error('tRPC client or required properties not available for full config');
      return;
    }
    
    // Get initial value
    try {
      window.trpc.config.getAll.query()
        .then(setConfig)
        .catch(err => console.error('Failed to get full config:', err));
    } catch (err) {
      console.error('Error invoking config.getAll.query:', err);
    }

    // Subscribe to all config changes
    try {
      if (!window.trpc.config.onConfigChange) {
        console.error('tRPC config.onConfigChange not available');
        return () => {};
      }
      
      const subscription = window.trpc.config.onConfigChange.subscribe();
      
      if (!subscription || typeof subscription.subscribe !== 'function') {
        console.error('Invalid subscription object returned for full config');
        return () => {};
      }
      
      const unsubscribe = subscription.subscribe({
        next: () => {
          // Refresh the full config when any change occurs
          try {
            window.trpc.config.getAll.query().then(setConfig);
          } catch (err) {
            console.error('Error refreshing full config after change:', err);
          }
        },
        error: (err) => console.error('Error in full config subscription:', err)
      });

      return () => {
        try {
          unsubscribe.unsubscribe();
        } catch (err) {
          console.error('Error unsubscribing from full config:', err);
        }
      };
    } catch (err) {
      console.error('Failed to subscribe to config changes:', err);
      return () => {};
    }
  }, []);

  return config;
}

/**
 * Hook to subscribe to events via tRPC
 */
export function useTrpcEvents<T = any>() {
  const [events, setEvents] = useState<T[]>([]);
  
  useEffect(() => {
    if (!window.trpc || !window.trpc.events || !window.trpc.events.onEvent) {
      console.error('tRPC client or events.onEvent not available');
      return;
    }
    
    try {
      const subscription = window.trpc.events.onEvent.subscribe();
      
      if (!subscription || typeof subscription.subscribe !== 'function') {
        console.error('Invalid subscription object returned for events');
        return () => {};
      }
      
      const unsubscribe = subscription.subscribe({
        next: (event) => {
          console.log('Received event:', event);
          setEvents(prev => [event as T, ...prev].slice(0, 20));
        },
        error: (err) => console.error('Error in events subscription:', err)
      });
      
      return () => {
        try {
          unsubscribe.unsubscribe();
        } catch (err) {
          console.error('Error unsubscribing from events:', err);
        }
      };
    } catch (err) {
      console.error('Failed to subscribe to events:', err);
      return () => {};
    }
  }, []);
  
  return events;
}

/**
 * Hook to get all adapters via tRPC
 */
export function useTrpcAdapters() {
  const [adapters, setAdapters] = useState<RouterOutput['adapters']['getAll']>([]);
  
  useEffect(() => {
    if (!window.trpc) {
      console.error('tRPC client not available');
      return;
    }
    
    const fetchAdapters = async () => {
      try {
        const result = await window.trpc.adapters.getAll.query();
        setAdapters(result);
      } catch (error) {
        console.error('Error fetching adapters:', error);
      }
    };
    
    fetchAdapters();
    
    // Refresh adapters when config changes (this is a simplification)
    try {
      const subscription = window.trpc.config.onConfigChange.subscribe();
      const unsubscribe = subscription.subscribe({
        next: (change) => {
          if (change.key.startsWith('adapters')) {
            fetchAdapters();
          }
        },
        error: (err) => console.error('Error in adapter subscription:', err)
      });
      
      return () => {
        unsubscribe.unsubscribe();
      };
    } catch (err) {
      console.error('Failed to subscribe to config changes for adapters:', err);
      return () => {};
    }
  }, []);
  
  return adapters;
}

/**
 * Hook to create an adapter via tRPC
 */
export function useCreateAdapter() {
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  
  const createAdapter = async (config: RouterInput['adapters']['create']) => {
    if (!window.trpc) {
      const err = new Error('tRPC client not available');
      setError(err);
      throw err;
    }
    
    setIsCreating(true);
    setError(null);
    
    try {
      const result = await window.trpc.adapters.create.mutate(config);
      return result;
    } catch (err) {
      const error = err as Error;
      setError(error);
      throw error;
    } finally {
      setIsCreating(false);
    }
  };
  
  return { createAdapter, isCreating, error };
}

/**
 * Hook to control an adapter via tRPC
 */
export function useAdapterControl(adapterId: string) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  
  const checkTrpc = () => {
    if (!window.trpc) {
      const err = new Error('tRPC client not available');
      setError(err);
      throw err;
    }
  };
  
  const startAdapter = async () => {
    checkTrpc();
    setIsLoading(true);
    setError(null);
    
    try {
      await window.trpc.adapters.start.mutate(adapterId);
    } catch (err) {
      const error = err as Error;
      setError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };
  
  const stopAdapter = async () => {
    checkTrpc();
    setIsLoading(true);
    setError(null);
    
    try {
      await window.trpc.adapters.stop.mutate(adapterId);
    } catch (err) {
      const error = err as Error;
      setError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };
  
  const updateAdapter = async (config: Record<string, any>) => {
    checkTrpc();
    setIsLoading(true);
    setError(null);
    
    try {
      await window.trpc.adapters.update.mutate({ id: adapterId, config });
    } catch (err) {
      const error = err as Error;
      setError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };
  
  const deleteAdapter = async () => {
    checkTrpc();
    setIsLoading(true);
    setError(null);
    
    try {
      await window.trpc.adapters.delete.mutate(adapterId);
    } catch (err) {
      const error = err as Error;
      setError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };
  
  return { 
    startAdapter, 
    stopAdapter, 
    updateAdapter, 
    deleteAdapter,
    isLoading,
    error
  };
}