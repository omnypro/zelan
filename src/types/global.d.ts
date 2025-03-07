/**
 * Global types for the application
 */

interface Window {
  // Add window properties available in the Electron renderer process
  zelan?: {
    adapters: {
      getStatus: (adapterId: string) => Promise<any>;
      connect: (adapterId: string) => Promise<any>;
      disconnect: (adapterId: string) => Promise<any>;
      updateConfig: (adapterId: string, config: any) => Promise<any>;
    };
    websocket: {
      getStatus: () => Promise<any>;
      start: () => Promise<any>;
      stop: () => Promise<any>;
      updateConfig: (config: any) => Promise<any>;
    };
    auth: {
      getState: (serviceId: string) => Promise<any>;
      authenticate: (serviceId: string) => Promise<any>;
      logout: (serviceId: string) => Promise<any>;
    };
    events: {
      getRecentEvents: (count?: number) => Promise<any>;
    };
    config: {
      getAdapterSettings: (adapterId: string) => Promise<any>;
      updateAdapterSettings: (adapterId: string, settings: Record<string, any>) => Promise<any>;
      getAllAdapterSettings: () => Promise<any>;
      setAdapterEnabled: (adapterId: string, enabled: boolean) => Promise<any>;
      setAdapterAutoConnect: (adapterId: string, autoConnect: boolean) => Promise<any>;
      getAppConfig: () => Promise<any>;
      updateAppConfig: (config: Record<string, any>) => Promise<any>;
      getUserData: () => Promise<any>;
      updateUserData: (data: Record<string, any>) => Promise<any>;
      getToken: (serviceId: string) => Promise<any>;
      saveToken: (serviceId: string, token: Record<string, any>) => Promise<any>;
      deleteToken: (serviceId: string) => Promise<any>;
      hasValidToken: (serviceId: string) => Promise<any>;
      clearAllTokens: () => Promise<any>;
    };
  };
  
  // tRPC bridge
  trpcBridge?: {
    request: (path: string, type: 'query' | 'mutation', input: unknown) => Promise<unknown>;
  };
}