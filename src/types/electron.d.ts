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

export interface AdapterResult {
  success: boolean;
  error?: string;
}

export interface WebSocketStatus {
  isRunning: boolean;
  clientCount: number;
}

export interface WebSocketResult {
  success: boolean;
  error?: string;
}

export interface AuthState {
  state: string;
  isAuthenticated: boolean;
}

export interface AuthResult {
  success: boolean;
  error?: string;
}

export interface EventsResult {
  events: any[];
}

export interface ZelanApi {
  adapters: {
    getStatus: (adapterId: string) => Promise<AdapterStatus>;
    connect: (adapterId: string) => Promise<AdapterResult>;
    disconnect: (adapterId: string) => Promise<AdapterResult>;
    updateConfig: (adapterId: string, config: any) => Promise<AdapterResult>;
  };
  
  websocket: {
    getStatus: () => Promise<WebSocketStatus>;
    start: () => Promise<WebSocketResult>;
    stop: () => Promise<WebSocketResult>;
    updateConfig: (config: any) => Promise<WebSocketResult>;
  };
  
  auth: {
    getState: (serviceId: string) => Promise<AuthState>;
    authenticate: (serviceId: string) => Promise<AuthResult>;
    logout: (serviceId: string) => Promise<AuthResult>;
  };
  
  events: {
    getRecentEvents: (count?: number) => Promise<EventsResult>;
  };
}

declare global {
  interface Window {
    ipcRenderer: IpcRenderer;
    zelan: ZelanApi;
  }
}