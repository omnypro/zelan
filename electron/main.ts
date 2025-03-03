import { app, BrowserWindow, ipcMain } from 'electron'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { bootstrap, shutdown } from '../src/lib/core/bootstrap'
import { appRouter } from '../src/lib/trpc/server/router'
import { inferRouterInputs, inferRouterOutputs } from '@trpc/server'
import type { AppRouter } from '../src/lib/trpc/server/router'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

process.env.APP_ROOT = path.join(__dirname, '..')

// 🚧 Use ['ENV_NAME'] avoid vite:define plugin - Vite@2.x
export const VITE_DEV_SERVER_URL = process.env['VITE_DEV_SERVER_URL']
export const MAIN_DIST = path.join(process.env.APP_ROOT, 'dist-electron')
export const RENDERER_DIST = path.join(process.env.APP_ROOT, 'dist')

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL
  ? path.join(process.env.APP_ROOT, 'public')
  : RENDERER_DIST

let win: BrowserWindow | null
let coreInitialized = false

// Define types for tRPC router - these will be used in the future
// eslint-disable-next-line @typescript-eslint/no-unused-vars
type RouterInput = inferRouterInputs<AppRouter>;
// eslint-disable-next-line @typescript-eslint/no-unused-vars 
type RouterOutput = inferRouterOutputs<AppRouter>;

// Initialize the core services
async function initializeCore() {
  if (coreInitialized) return

  try {
    // Bootstrap with only test adapter for now, no WebSocket server
    await bootstrap({
      enableTestAdapter: true,
      startWebSocketServer: false
    })

    console.log('Core services initialized')
    coreInitialized = true
  } catch (error) {
    console.error('Failed to initialize core services:', error)
  }
}

// Shutdown the core services
async function shutdownCore() {
  if (!coreInitialized) return

  try {
    await shutdown()
    console.log('Core services shut down')
    coreInitialized = false
  } catch (error) {
    console.error('Failed to shut down core services:', error)
  }
}

function createWindow() {
  win = new BrowserWindow({
    icon: path.join(process.env.VITE_PUBLIC, 'electron-vite.svg'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.mjs'),
      nodeIntegration: false,
      contextIsolation: true
    },
    width: 1200,
    height: 800
  })

  // Initialize core services
  initializeCore().catch(console.error)

  // Test active push message to Renderer-process.
  win.webContents.on('did-finish-load', () => {
    win?.webContents.send('main-process-message', new Date().toLocaleString())
  })

  if (VITE_DEV_SERVER_URL) {
    win.loadURL(VITE_DEV_SERVER_URL)
  } else {
    win.loadFile(path.join(RENDERER_DIST, 'index.html'))
  }
}

// Handler implementations for direct calls
// These handlers directly implement the functionality instead of calling tRPC procedures
const handlers = {
  // Adapter handlers
  getAdapterStatus: async (adapterId: string) => {
    const { AdapterManager } = await import('../src/lib/core/adapters')
    const adapterManager = AdapterManager.getInstance()
    const adapter = adapterManager.getAdapter(adapterId)

    if (!adapter) {
      return { status: 'not-found', isConnected: false }
    }

    return {
      status: adapter.state,
      isConnected: adapter.isConnected(),
      config: adapter.config
    }
  },

  connectAdapter: async (adapterId: string) => {
    const { AdapterManager } = await import('../src/lib/core/adapters')
    const adapterManager = AdapterManager.getInstance()
    const adapter = adapterManager.getAdapter(adapterId)

    if (!adapter) {
      return { success: false, error: 'Adapter not found' }
    }

    try {
      await adapter.connect()
      return { success: true }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return { success: false, error: errorMessage }
    }
  },

  disconnectAdapter: async (adapterId: string) => {
    const { AdapterManager } = await import('../src/lib/core/adapters')
    const adapterManager = AdapterManager.getInstance()
    const adapter = adapterManager.getAdapter(adapterId)

    if (!adapter) {
      return { success: false, error: 'Adapter not found' }
    }

    try {
      await adapter.disconnect()
      return { success: true }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return { success: false, error: errorMessage }
    }
  },

  updateAdapterConfig: async (adapterId: string, config: Record<string, unknown>) => {
    const { AdapterManager } = await import('../src/lib/core/adapters')
    const adapterManager = AdapterManager.getInstance()
    const adapter = adapterManager.getAdapter(adapterId)

    if (!adapter) {
      return { success: false, error: 'Adapter not found' }
    }

    try {
      adapter.updateConfig(config)
      return { success: true }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return { success: false, error: errorMessage }
    }
  },

  // WebSocket server handlers
  getWebSocketStatus: async () => {
    return {
      isRunning: false,
      clientCount: 0
    }
  },

  startWebSocketServer: async () => {
    return {
      success: false,
      error: 'WebSocket server is disabled'
    }
  },

  stopWebSocketServer: async () => {
    return {
      success: true
    }
  },

  updateWebSocketConfig: async (config: Record<string, unknown>) => {
    return {
      success: false,
      error: 'WebSocket server is disabled'
    }
  },

  // Auth handlers
  getAuthState: async (serviceId: string) => {
    const { AuthService, AuthState } = await import('../src/lib/core/auth')
    const authService = AuthService.getInstance()

    return {
      state: authService.getAuthState(serviceId),
      isAuthenticated: authService.getAuthState(serviceId) === AuthState.AUTHENTICATED
    }
  },

  authenticate: async (serviceId: string) => {
    const { AuthService } = await import('../src/lib/core/auth')
    const authService = AuthService.getInstance()

    try {
      await authService.authenticate(serviceId)
      return { success: true }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return { success: false, error: errorMessage }
    }
  },

  logout: async (serviceId: string) => {
    const { AuthService } = await import('../src/lib/core/auth')
    const authService = AuthService.getInstance()

    try {
      await authService.logout(serviceId)
      return { success: true }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return { success: false, error: errorMessage }
    }
  },

  // Event handlers
  getRecentEvents: async (count = 10) => {
    return {
      events: []
    }
  }
}

// Register tRPC and IPC handlers
function registerHandlers() {
  // Increase the maximum number of listeners to avoid memory leak warnings
  ipcMain.setMaxListeners(20);
  // Single entry point for all tRPC calls
  ipcMain.handle('trpc', async (_event, req) => {
    const { path, type, input } = req;
    
    // Split the path by dot to get the procedure path
    const [namespace, procedure] = path.split('.');
    
    console.log(`tRPC ${type} request: ${path}`, input);
    
    try {
      let result;
      
      // Instead of accessing the tRPC router directly, map to our handlers
      switch (`${namespace}.${procedure}`) {
        case 'adapter.getStatus':
          result = await handlers.getAdapterStatus(input);
          break;
        case 'adapter.connect':
          result = await handlers.connectAdapter(input);
          break;
        case 'adapter.disconnect':
          result = await handlers.disconnectAdapter(input);
          break;
        case 'adapter.updateConfig':
          result = await handlers.updateAdapterConfig(input.adapterId, input.config);
          break;
        case 'websocket.getStatus':
          result = await handlers.getWebSocketStatus();
          break;
        case 'websocket.start':
          result = await handlers.startWebSocketServer();
          break;
        case 'websocket.stop':
          result = await handlers.stopWebSocketServer();
          break;
        case 'websocket.updateConfig':
          result = await handlers.updateWebSocketConfig(input);
          break;
        case 'auth.getState':
          result = await handlers.getAuthState(input);
          break;
        case 'auth.authenticate':
          result = await handlers.authenticate(input);
          break;
        case 'auth.logout':
          result = await handlers.logout(input);
          break;
        case 'event.getRecentEvents':
          result = await handlers.getRecentEvents(input);
          break;
        default:
          throw new Error(`Unknown procedure: ${path}`);
      }
      
      console.log(`tRPC ${type} response for ${path}:`, result);
      return result;
    } catch (error) {
      console.error('tRPC handler error:', error);
      // Properly format error for tRPC client
      return {
        error: {
          message: error instanceof Error ? error.message : String(error),
          code: 'INTERNAL_SERVER_ERROR'
        }
      };
    }
  });
  
  // Legacy IPC handlers for backward compatibility
  ipcMain.handle('get-adapter-status', async (_event, adapterId) => {
    return await handlers.getAdapterStatus(adapterId);
  });

  ipcMain.handle('connect-adapter', async (_event, adapterId) => {
    return await handlers.connectAdapter(adapterId);
  });

  ipcMain.handle('disconnect-adapter', async (_event, adapterId) => {
    return await handlers.disconnectAdapter(adapterId);
  });

  ipcMain.handle('update-adapter-config', async (_event, adapterId, config) => {
    return await handlers.updateAdapterConfig(adapterId, config);
  });

  ipcMain.handle('get-websocket-status', async () => {
    return await handlers.getWebSocketStatus();
  });

  ipcMain.handle('start-websocket-server', async () => {
    return await handlers.startWebSocketServer();
  });

  ipcMain.handle('stop-websocket-server', async () => {
    return await handlers.stopWebSocketServer();
  });

  ipcMain.handle('update-websocket-config', async (_event, config) => {
    return await handlers.updateWebSocketConfig(config);
  });

  ipcMain.handle('get-auth-state', async (_event, serviceId) => {
    return await handlers.getAuthState(serviceId);
  });

  ipcMain.handle('authenticate', async (_event, serviceId) => {
    return await handlers.authenticate(serviceId);
  });

  ipcMain.handle('logout', async (_event, serviceId) => {
    return await handlers.logout(serviceId);
  });

  ipcMain.handle('get-recent-events', async (_event, count = 10) => {
    return await handlers.getRecentEvents(count);
  });
}

// Quit when all windows are closed, except on macOS. There, it's common
// for applications and their menu bar to stay active until the user quits
// explicitly with Cmd + Q.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    shutdownCore().finally(() => {
      app.quit()
      win = null
    })
  }
})

app.on('activate', () => {
  // On OS X it's common to re-create a window in the app when the
  // dock icon is clicked and there are no other windows open.
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})

app.on('before-quit', async (event) => {
  if (coreInitialized) {
    event.preventDefault()
    await shutdownCore()
    app.quit()
  }
})

app.whenReady().then(() => {
  registerHandlers()
  createWindow()
})