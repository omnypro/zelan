import { useCallback } from 'react';
import { useTauriCommand } from './useTauriCommand';
import { 
  EventBusStats, 
  AdapterStatusMap, 
  AdapterSettingsMap, 
  WebSocketInfo,
  ZelanError
} from '../types';

interface UseDataFetchingOptions {
  onError?: (error: ZelanError | string) => void;
  onSuccess?: () => void;
}

/**
 * A hook for fetching application data from Tauri backend
 */
export const useDataFetching = (options?: UseDataFetchingOptions) => {
  const { invoke } = useTauriCommand({
    onError: options?.onError
  });

  // Fetch event bus stats
  const fetchEventBusStats = useCallback(async (): Promise<EventBusStats> => {
    return await invoke<EventBusStats>('get_event_bus_status');
  }, [invoke]);

  // Fetch adapter statuses
  const fetchAdapterStatuses = useCallback(async (): Promise<AdapterStatusMap> => {
    return await invoke<AdapterStatusMap>('get_adapter_statuses');
  }, [invoke]);

  // Fetch adapter settings
  const fetchAdapterSettings = useCallback(async (): Promise<AdapterSettingsMap> => {
    return await invoke<AdapterSettingsMap>('get_adapter_settings');
  }, [invoke]);

  // Fetch WebSocket info
  const fetchWebSocketInfo = useCallback(async (): Promise<WebSocketInfo> => {
    return await invoke<WebSocketInfo>('get_websocket_info');
  }, [invoke]);

  // Send a test event
  const sendTestEvent = useCallback(async (): Promise<string> => {
    return await invoke<string>('send_test_event');
  }, [invoke]);

  // Update WebSocket port
  const updateWebSocketPort = useCallback(async (port: number): Promise<string> => {
    return await invoke<string>('set_websocket_port', { port });
  }, [invoke]);

  // Fetch all data at once
  const fetchAllData = useCallback(async () => {
    try {
      const [stats, statuses, settings, info] = await Promise.all([
        fetchEventBusStats(),
        fetchAdapterStatuses(),
        fetchAdapterSettings(),
        fetchWebSocketInfo()
      ]);

      if (options?.onSuccess) {
        options.onSuccess();
      }

      return { stats, statuses, settings, info };
    } catch (error) {
      // Error is already handled by the useTauriCommand hook
      throw error;
    }
  }, [
    fetchEventBusStats, 
    fetchAdapterStatuses, 
    fetchAdapterSettings, 
    fetchWebSocketInfo, 
    options
  ]);

  return {
    fetchEventBusStats,
    fetchAdapterStatuses,
    fetchAdapterSettings,
    fetchWebSocketInfo,
    fetchAllData,
    sendTestEvent,
    updateWebSocketPort,
  };
};