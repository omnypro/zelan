/**
 * Type definitions for the Electron API exposed to the renderer process
 */

export interface IpcRenderer {
  on: (channel: string, listener: (...args: any[]) => void) => void;
  off: (channel: string, listener: (...args: any[]) => void) => void;
  send: (channel: string, ...args: any[]) => void;
  invoke: (channel: string, ...args: any[]) => Promise<any>;
}

export interface AdapterStatus {
  status: string;
  isConnected: boolean;
  config: Record<string, any>;
}

export interface OperationResult {
  success: boolean;
  error?: string;
}

export interface WebSocketStatus {
  isRunning: boolean;
  clientCount: number;
}

export interface AuthState {
  state: string;
  isAuthenticated: boolean;
}

export interface EventsResult {
  events: any[];
}

export interface ConfigResponse {
  success: boolean;
  data?: Record<string, any>;
  error?: string;
}

export interface ZelanApi {
  adapters: {
    getStatus: (adapterId: string) => Promise<AdapterStatus>;
    connect: (adapterId: string) => Promise<OperationResult>;
    disconnect: (adapterId: string) => Promise<OperationResult>;
    updateConfig: (adapterId: string, config: any) => Promise<OperationResult>;
  };
  
  websocket: {
    getStatus: () => Promise<WebSocketStatus>;
    start: () => Promise<OperationResult>;
    stop: () => Promise<OperationResult>;
    updateConfig: (config: any) => Promise<OperationResult>;
  };
  
  auth: {
    getState: (serviceId: string) => Promise<AuthState>;
    authenticate: (serviceId: string) => Promise<OperationResult>;
    logout: (serviceId: string) => Promise<OperationResult>;
  };
  
  events: {
    getRecentEvents: (count?: number) => Promise<EventsResult>;
  };
  
  config: {
    getAdapterSettings: (adapterId: string) => Promise<ConfigResponse>;
    updateAdapterSettings: (adapterId: string, settings: Record<string, any>) => Promise<OperationResult>;
    getAllAdapterSettings: () => Promise<ConfigResponse>;
    setAdapterEnabled: (adapterId: string, enabled: boolean) => Promise<OperationResult>;
    setAdapterAutoConnect: (adapterId: string, autoConnect: boolean) => Promise<OperationResult>;
    getAppConfig: () => Promise<ConfigResponse>;
    updateAppConfig: (config: Record<string, any>) => Promise<OperationResult>;
    getUserData: () => Promise<ConfigResponse>;
    updateUserData: (data: Record<string, any>) => Promise<OperationResult>;
  };
}

declare global {
  interface Window {
    ipcRenderer: IpcRenderer;
    zelan: ZelanApi;
  }
}