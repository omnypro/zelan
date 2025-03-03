import { ipcRenderer, contextBridge } from 'electron'
import type { AdapterStatus, OperationResult, WebSocketStatus, AuthState, EventsResponse, WebSocketConfig } from '../src/lib/trpc/shared/types'

// --------- Expose tRPC bridge to the Renderer process ---------
contextBridge.exposeInMainWorld('trpcBridge', {
  // Generic tRPC request handler
  request: async (path: string, type: 'query' | 'mutation', input: unknown) => {
    return ipcRenderer.invoke('trpc', { path, type, input })
  }
})

// --------- Expose general IPC to the Renderer process ---------
// Create a map to store wrapped listeners so we can properly remove them
const listenerMap = new Map();

contextBridge.exposeInMainWorld('ipcRenderer', {
  on(channel: string, listener: (...args: any[]) => void) {
    // Create a wrapped listener that will be used with the actual ipcRenderer
    const wrappedListener = (_event: Electron.IpcRendererEvent, ...args: any[]) => listener(...args);
    
    // Store the wrapped listener so we can retrieve it later for removal
    if (!listenerMap.has(channel)) {
      listenerMap.set(channel, new Map());
    }
    listenerMap.get(channel).set(listener, wrappedListener);
    
    // Add the actual listener
    ipcRenderer.on(channel, wrappedListener);
    
    return this;
  },
  
  off(channel: string, listener: (...args: any[]) => void) {
    // Get the channel-specific listener map
    const channelListeners = listenerMap.get(channel);
    if (channelListeners && channelListeners.has(listener)) {
      // Get the original wrapped listener
      const wrappedListener = channelListeners.get(listener);
      
      // Remove the actual listener
      ipcRenderer.off(channel, wrappedListener);
      
      // Clean up our listener map
      channelListeners.delete(listener);
      if (channelListeners.size === 0) {
        listenerMap.delete(channel);
      }
    }
    
    return this;
  },
  
  send(channel: string, ...args: any[]) {
    return ipcRenderer.send(channel, ...args);
  },
  
  invoke(channel: string, ...args: any[]) {
    return ipcRenderer.invoke(channel, ...args);
  },
})

// Expose the Zelan API to the renderer process (for backward compatibility)
contextBridge.exposeInMainWorld('zelan', {
  // Adapter functions
  adapters: {
    getStatus: (adapterId: string): Promise<AdapterStatus> => {
      return ipcRenderer.invoke('get-adapter-status', adapterId)
    },
    connect: (adapterId: string): Promise<OperationResult> => {
      return ipcRenderer.invoke('connect-adapter', adapterId)
    },
    disconnect: (adapterId: string): Promise<OperationResult> => {
      return ipcRenderer.invoke('disconnect-adapter', adapterId)
    },
    updateConfig: (adapterId: string, config: Record<string, unknown>): Promise<OperationResult> => {
      return ipcRenderer.invoke('update-adapter-config', adapterId, config)
    },
  },
  
  // WebSocket server functions
  websocket: {
    getStatus: (): Promise<WebSocketStatus> => {
      return ipcRenderer.invoke('get-websocket-status')
    },
    start: (): Promise<OperationResult> => {
      return ipcRenderer.invoke('start-websocket-server')
    },
    stop: (): Promise<OperationResult> => {
      return ipcRenderer.invoke('stop-websocket-server')
    },
    updateConfig: (config: WebSocketConfig): Promise<OperationResult> => {
      return ipcRenderer.invoke('update-websocket-config', config)
    },
  },
  
  // Auth functions
  auth: {
    getState: (serviceId: string): Promise<AuthState> => {
      return ipcRenderer.invoke('get-auth-state', serviceId)
    },
    authenticate: (serviceId: string): Promise<OperationResult> => {
      return ipcRenderer.invoke('authenticate', serviceId)
    },
    logout: (serviceId: string): Promise<OperationResult> => {
      return ipcRenderer.invoke('logout', serviceId)
    },
  },
  
  // Event functions
  events: {
    getRecentEvents: (count: number = 10): Promise<EventsResponse> => {
      return ipcRenderer.invoke('get-recent-events', count)
    },
  },
})