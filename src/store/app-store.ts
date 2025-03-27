import { createStore } from '@tanstack/store'
import { AdapterStatus } from '../types'

// Define the shape of our application state
interface AppState {
  // UI state
  theme: 'light' | 'dark' | 'system'
  sidebarCollapsed: boolean
  
  // Connection state
  isConnected: boolean
  
  // Adapter states
  adapters: {
    [key: string]: {
      status: AdapterStatus
      lastConnected?: string
      error?: string
    }
  }
  
  // WebSocket state
  websocket: {
    port: number
    active: boolean
    connections: number
  }
}

// Initial state
const initialState: AppState = {
  theme: 'system',
  sidebarCollapsed: false,
  isConnected: false,
  adapters: {
    twitch: { status: 'disconnected' },
    obs: { status: 'disconnected' },
    test: { status: 'connected', lastConnected: new Date().toISOString() }
  },
  websocket: {
    port: 9000,
    active: false,
    connections: 0
  }
}

// Create the store
export const appStore = createStore<AppState>()({
  state: initialState,
})

// Create actions
export const appActions = {
  // Theme actions
  setTheme: (theme: 'light' | 'dark' | 'system') => {
    appStore.setState(state => {
      state.theme = theme
    })
  },
  
  // Sidebar actions
  toggleSidebar: () => {
    appStore.setState(state => {
      state.sidebarCollapsed = !state.sidebarCollapsed
    })
  },
  
  // Connection actions
  setConnectionStatus: (isConnected: boolean) => {
    appStore.setState(state => {
      state.isConnected = isConnected
    })
  },
  
  // Adapter actions
  updateAdapterStatus: (adapterId: string, status: AdapterStatus, error?: string) => {
    appStore.setState(state => {
      if (!state.adapters[adapterId]) {
        state.adapters[adapterId] = { status }
      } else {
        state.adapters[adapterId].status = status
        
        if (status === 'connected') {
          state.adapters[adapterId].lastConnected = new Date().toISOString()
          state.adapters[adapterId].error = undefined
        } else if (status === 'error' && error) {
          state.adapters[adapterId].error = error
        }
      }
    })
  },
  
  // WebSocket actions
  updateWebSocketInfo: (port: number, active: boolean, connections: number) => {
    appStore.setState(state => {
      state.websocket = {
        port,
        active,
        connections
      }
    })
  }
}

// Export selectors
export const appSelectors = {
  getTheme: () => appStore.state.theme,
  isSidebarCollapsed: () => appStore.state.sidebarCollapsed,
  isConnected: () => appStore.state.isConnected,
  getAdapters: () => appStore.state.adapters,
  getAdapter: (adapterId: string) => appStore.state.adapters[adapterId],
  getWebSocketInfo: () => appStore.state.websocket
}