interface Window {
  trpc: {
    config: {
      get: {
        query: (params: { key: string; defaultValue?: any }) => Promise<any>
      }
      set: {
        mutate: (params: { key: string; value: any }) => Promise<boolean>
      }
      update: {
        mutate: (updates: Record<string, any>) => Promise<boolean>
      }
      delete: {
        mutate: (key: string) => Promise<boolean>
      }
      has: {
        query: (key: string) => Promise<boolean>
      }
      getAll: {
        query: () => Promise<Record<string, any>>
      }
      getPath: {
        query: () => Promise<string>
      }
      onConfigChange: {
        subscribe: () => { subscribe: Function }
      }
      onPathChange: {
        subscribe: (path: string) => { subscribe: Function }
      }
    }

    events: {
      send: {
        mutate: (event: any) => Promise<boolean>
      }
      onEvent: {
        subscribe: () => { subscribe: Function }
      }
    }

    adapters: {
      getAll: {
        query: () => Promise<any[]>
      }
      getById: {
        query: (id: string) => Promise<any>
      }
      create: {
        mutate: (config: any) => Promise<any>
      }
      start: {
        mutate: (id: string) => Promise<boolean>
      }
      stop: {
        mutate: (id: string) => Promise<boolean>
      }
      update: {
        mutate: (params: { id: string; config: Record<string, any> }) => Promise<boolean>
      }
      delete: {
        mutate: (id: string) => Promise<boolean>
      }
      getTypes: {
        query: () => Promise<string[]>
      }
    }

    websocket: {
      getStatus: {
        query: () => Promise<{
          running: boolean
          clientCount: number
          port: number
        }>
      }
      start: {
        mutate: () => Promise<boolean>
      }
      stop: {
        mutate: () => Promise<boolean>
      }
    }
    
    reconnection: {
      getStatus: {
        query: (adapterId: string) => Promise<{
          enabled: boolean
          interval: number
          maxAttempts: number
          attempts: number
          isRetrying: boolean
        }>
      }
      enable: {
        mutate: (adapterId: string) => Promise<boolean>
      }
      disable: {
        mutate: (adapterId: string) => Promise<boolean>
      }
      setInterval: {
        mutate: (params: { id: string; interval: number }) => Promise<boolean>
      }
      setMaxAttempts: {
        mutate: (params: { id: string; maxAttempts: number }) => Promise<boolean>
      }
      resetAttempts: {
        mutate: (adapterId: string) => Promise<boolean>
      }
    }
    
    auth: {
      getStatus: {
        query: (provider: string) => Promise<any>
      }
      isAuthenticated: {
        query: (provider: string) => Promise<boolean>
      }
      authenticate: {
        mutate: (params: { provider: string; options: any }) => Promise<any>
      }
      refreshToken: {
        mutate: (provider: string) => Promise<any>
      }
      revokeToken: {
        mutate: (provider: string) => Promise<boolean>
      }
      onStatusChange: {
        subscribe: (provider: string) => { subscribe: Function }
      }
      onDeviceCode: {
        subscribe: () => { subscribe: Function }
      }
    }
  }

  electron: {
    ipcRenderer: {
      send: (channel: string, ...args: any[]) => void
      on: (channel: string, func: (...args: any[]) => void) => void
      once: (channel: string, func: (...args: any[]) => void) => void
      invoke: (channel: string, ...args: any[]) => Promise<any>
    }
  }
}
