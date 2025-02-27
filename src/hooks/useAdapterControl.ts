import { useCallback } from 'react';
import { AdapterSettings, ZelanError } from '../types';
import { useTauriCommand } from './useTauriCommand';

interface UseAdapterControlOptions {
  onSuccess?: (message: string) => void;
  onError?: (error: ZelanError | string) => void;
}

/**
 * A hook for controlling adapters (enable/disable, configure)
 */
export const useAdapterControl = (options?: UseAdapterControlOptions) => {
  const { invoke } = useTauriCommand({
    onError: options?.onError
  });

  // Toggle adapter enabled status
  const toggleAdapterEnabled = useCallback(async (
    adapterName: string,
    settings: AdapterSettings
  ) => {
    // Create updated settings with toggled enabled status
    const updatedSettings: AdapterSettings = {
      ...settings,
      enabled: !settings.enabled,
    };

    // Update settings on the backend
    await invoke('update_adapter_settings', {
      adapterName,
      settings: updatedSettings,
    });

    // If enabling, connect the adapter
    if (updatedSettings.enabled) {
      await invoke('connect_adapter', {
        adapterName,
      });
    } else {
      // If disabling, disconnect the adapter
      await invoke('disconnect_adapter', {
        adapterName,
      });
    }

    // Call success callback if provided
    if (options?.onSuccess) {
      options.onSuccess(`${updatedSettings.enabled ? 'Enabled' : 'Disabled'} adapter: ${settings.display_name}`);
    }

    return updatedSettings;
  }, [invoke, options]);

  // Update adapter configuration
  const updateAdapterConfig = useCallback(async (
    adapterName: string,
    currentSettings: AdapterSettings,
    configUpdates: Record<string, any>
  ) => {
    // Create updated settings with merged config
    const updatedSettings: AdapterSettings = {
      ...currentSettings,
      config: {
        ...currentSettings.config,
        ...configUpdates,
      },
    };

    // Update settings on the backend
    await invoke('update_adapter_settings', {
      adapterName,
      settings: updatedSettings,
    });

    // Call success callback if provided
    if (options?.onSuccess) {
      options.onSuccess(`Updated configuration for: ${currentSettings.display_name}`);
    }

    return updatedSettings;
  }, [invoke, options]);

  return {
    toggleAdapterEnabled,
    updateAdapterConfig
  };
};