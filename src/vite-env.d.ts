/// <reference types="vite/client" />

import type { 
  AdapterStatus, 
  OperationResult, 
  WebSocketStatus, 
  AuthState as AuthStateType, 
  EventsResponse,
  WebSocketConfig 
} from './lib/trpc/shared/types';

declare global {
  interface Window {
    ipcRenderer: {
      send(channel: string, ...args: any[]): void;
      invoke(channel: string, ...args: any[]): Promise<any>;
      on(channel: string, listener: (event: any, ...args: any[]) => void): void;
      off(channel: string, listener: Function): void;
    };
    
    trpcBridge: {
      request(path: string, type: 'query' | 'mutation', input: unknown): Promise<unknown>;
    };
    
    zelan: {
      adapters: {
        getStatus: (adapterId: string) => Promise<AdapterStatus>;
        connect: (adapterId: string) => Promise<OperationResult>;
        disconnect: (adapterId: string) => Promise<OperationResult>;
        updateConfig: (adapterId: string, config: Record<string, unknown>) => Promise<OperationResult>;
      };
      
      websocket: {
        getStatus: () => Promise<WebSocketStatus>;
        start: () => Promise<OperationResult>;
        stop: () => Promise<OperationResult>;
        updateConfig: (config: WebSocketConfig) => Promise<OperationResult>;
      };
      
      auth: {
        getState: (serviceId: string) => Promise<AuthStateType>;
        authenticate: (serviceId: string) => Promise<OperationResult>;
        logout: (serviceId: string) => Promise<OperationResult>;
      };
      
      events: {
        getRecentEvents: (count?: number) => Promise<EventsResponse>;
      };
    };
  }
}