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
  };
}