import { app, shell, BrowserWindow, ipcMain } from 'electron'
import { join } from 'path'
import { readFileSync } from 'fs'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'

// Create a simple console logger for startup
// We'll use the full logging service after it's properly imported
const consoleLogger = {
  info: (message: string, meta?: Record<string, any>) => {
    console.info(`[Main] ${message}`, meta || '')
  },
  error: (message: string, meta?: Record<string, any>) => {
    console.error(`[Main] ${message}`, meta || '')
  },
  warn: (message: string, meta?: Record<string, any>) => {
    console.warn(`[Main] ${message}`, meta || '')
  },
  debug: (message: string, meta?: Record<string, any>) => {
    console.debug(`[Main] ${message}`, meta || '')
  }
}

consoleLogger.info('Zelan application starting up')

// Load environment variables from .env file manually
try {
  // __dirname may be relative in dev mode, so use app.getAppPath()
  const appRoot = app.getAppPath()
  const envPath = join(appRoot, '.env')
  consoleLogger.debug('Looking for .env file', { path: envPath })
  const envContent = readFileSync(envPath, 'utf8')

  const loadedVars: Record<string, string> = {}

  envContent.split('\n').forEach((line) => {
    // Skip empty lines and comments
    if (!line || line.startsWith('#')) return

    // Parse KEY=VALUE format
    const [key, ...valueParts] = line.split('=')
    const value = valueParts.join('=').trim()

    if (key && value) {
      const trimmedKey = key.trim()
      process.env[trimmedKey] = value

      // Don't log sensitive values
      if (
        trimmedKey.includes('SECRET') ||
        trimmedKey.includes('KEY') ||
        trimmedKey.includes('TOKEN')
      ) {
        loadedVars[trimmedKey] = '******'
      } else {
        loadedVars[trimmedKey] = value
      }
    }
  })

  consoleLogger.info('Environment variables loaded from .env file', { vars: loadedVars })
} catch (error) {
  consoleLogger.error('Failed to load .env file', {
    error: error instanceof Error ? error.message : String(error)
  })
}

// Import our services
import { MainEventBus } from '@m/services/eventBus'
import { AdapterManager } from '@m/services/adapters'
import { WebSocketService } from '@m/services/websocket'
import { getErrorService } from '@m/services/errors'
import { getAuthService } from '@m/services/auth'
import { AuthProvider } from '@s/auth/interfaces'
import { AdapterRegistry } from '@s/adapters'
import { EventCache } from '@m/services/events/EventCache'
import { ConfigStore, getConfigStore } from '@s/core/config'
import { TestAdapterFactory } from '@m/adapters/test'
import { ObsAdapterFactory } from '@m/adapters/obs'
import { TwitchAdapterFactory } from '@m/adapters/twitch'
import { setupTRPCServer } from '@m/trpc'
import { SystemEventType, EventCategory } from '@s/types/events'
import { createSystemEvent } from '@s/core/events'
// We use these types in our error handling
import type { ErrorService } from '@m/services/errors'

// Import for subscription management
import { SubscriptionManager } from '@s/utils/subscription-manager'
import { getLoggingService } from '@m/services/logging'

// Initialize the full logging service now that it's imported
const logger = getLoggingService().createLogger('Main')
logger.info('Logging service initialized')

// Global references
let mainWindow: BrowserWindow | null = null
let configStore: ConfigStore | null = null
let eventCache: EventCache | null = null
let mainEventBus: MainEventBus | null = null
let errorService: ErrorService | null = null
let adapterRegistry: AdapterRegistry | null = null
let adapterManager: AdapterManager | null = null
let webSocketService: WebSocketService | null = null
let authService: any = null // Using 'any' temporarily
let subscriptionManager: SubscriptionManager = new SubscriptionManager()

function createWindow(): void {
  // Create the browser window.
  mainWindow = new BrowserWindow({
    width: 900,
    height: 670,
    show: false,
    autoHideMenuBar: true,
    ...(process.platform === 'linux' ? { icon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.mjs'),
      sandbox: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow!.show()

    // Connect event bus to this window
    if (mainEventBus && mainWindow) {
      mainEventBus.addWebContents(mainWindow.webContents)
    }
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  // HMR for renderer base on electron-vite cli.
  // Load the remote URL for development or the local html file for production.
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

/**
 * Set up IPC handlers for the config store
 */
function setupConfigIpcHandlers(config: ConfigStore): void {
  // Handler for getting config values
  ipcMain.handle('config:get', async (_, key: string, defaultValue?: unknown) => {
    return config.get(key, defaultValue)
  })

  // Handler for setting config values
  ipcMain.handle('config:set', async (_, key: string, value: unknown) => {
    config.set(key, value)
    return true
  })

  // Handler for checking if a key exists
  ipcMain.handle('config:has', async (_, key: string) => {
    return config.has(key)
  })

  // Handler for deleting a key
  ipcMain.handle('config:delete', async (_, key: string) => {
    config.delete(key)
    return true
  })

  // Handler for getting all config data
  ipcMain.handle('config:getAll', async () => {
    return config.getAll()
  })

  // Handler for updating multiple values
  ipcMain.handle('config:update', async (_, updates: Record<string, unknown>) => {
    config.update(updates)
    return true
  })

  // Handler for getting the config file path
  ipcMain.handle('config:path', async () => {
    return config.fileName
  })

  // Set up event forwarding for config changes
  const CONFIG_CHANGE_CHANNEL = 'zelan:config-change'

  // Subscribe to config changes and forward to renderer
  config.changes$().subscribe((event) => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send(CONFIG_CHANGE_CHANNEL, event)
    }
  })
}

/**
 * Initialize the application core services
 */
async function initializeServices(): Promise<void> {
  try {
    // Create the configuration store
    configStore = getConfigStore()

    // Setup IPC handlers for config
    setupConfigIpcHandlers(configStore)

    // Create event cache
    eventCache = new EventCache(configStore)

    // Initialize event bus
    mainEventBus = new MainEventBus(eventCache)

    // Initialize error service
    errorService = getErrorService(mainEventBus)

    // Add console error handler for development
    if (is.dev) {
      errorService.addHandler({
        handleError: () => {
          // Additional development-mode error handling could go here
          // (already logged to console by the error service)
        }
      })
    }

    // Publish startup event
    mainEventBus.publish(
      createSystemEvent(SystemEventType.STARTUP, 'Zelan application starting', 'info', {
        version: app.getVersion()
      })
    )

    // Set up adapter registry
    adapterRegistry = new AdapterRegistry()

    // Register adapter factories
    adapterRegistry.register(new TestAdapterFactory())
    adapterRegistry.register(new ObsAdapterFactory())
    adapterRegistry.register(new TwitchAdapterFactory(mainEventBus))

    // Initialize adapter manager
    adapterManager = new AdapterManager(adapterRegistry, mainEventBus, configStore)
    await adapterManager.initialize()

    // Create a test adapter if none exists
    const adapters = adapterManager.getAllAdapters()
    if (adapters.length === 0) {
      await adapterManager.createAdapter({
        id: 'test-adapter',
        type: 'test',
        name: 'Test Adapter',
        enabled: true,
        options: {
          eventInterval: 3000,
          simulateErrors: false,
          eventTypes: ['message', 'follow', 'subscription']
        }
      })

      // Check for existing adapters
      const existingAdapters = adapterManager.getAllAdapters()
      const hasTwitchAdapter = existingAdapters.some((adapter) => adapter.type === 'twitch')

      // Create Twitch adapter if authenticated and one doesn't already exist
      try {
        const authService = getAuthService(mainEventBus)
        const twitchAuthStatus = authService.getStatus(AuthProvider.TWITCH)

        if (twitchAuthStatus.state === 'authenticated' && !hasTwitchAdapter) {
          logger.info('Twitch authenticated at startup, creating Twitch adapter')
          await adapterManager.createAdapter({
            id: 'twitch-adapter',
            type: 'twitch',
            name: 'Twitch Adapter',
            enabled: true,
            options: {
              channelName: '', // Will use authenticated user's channel by default
              includeSubscriptions: false,
              eventsToTrack: [
                'channel.update',
                'stream.online',
                'stream.offline',
                'chat.message',
                'chat.cheer',
                'chat.subscription',
                'chat.raid'
              ]
            }
          })
        } else if (twitchAuthStatus.state === 'authenticated') {
          logger.info('Twitch adapter already exists')
        } else {
          logger.info('Twitch not authenticated, skipping Twitch adapter creation')
        }
      } catch (error) {
        logger.error('Error creating Twitch adapter', {
          error: error instanceof Error ? error.message : String(error)
        })
      }
    }

    // Initialize WebSocket service
    webSocketService = WebSocketService.getInstance(mainEventBus, configStore)

    // Get WebSocket settings from config or use defaults
    // We don't need to assign this value since it's retrieved automatically in the WebSocketService
    configStore.getSettings()

    // Start WebSocket server automatically
    if (webSocketService) {
      webSocketService.start()
    }

    // Initialize the auth service
    authService = getAuthService(mainEventBus)
    await authService.initialize()

    // Listen for authentication events to create adapters dynamically
    const authEventSubscription = mainEventBus.events$.subscribe(async (event) => {
      try {
        // Check for auth events from the TwitchAuthService
        if (
          event.category === EventCategory.SERVICE &&
          event.type === 'authenticated' &&
          event.payload &&
          typeof event.payload === 'object' &&
          'provider' in event.payload &&
          event.payload.provider === AuthProvider.TWITCH
        ) {
          logger.info('Twitch authentication successful, creating Twitch adapter')

          // Check if a Twitch adapter already exists
          if (adapterManager) {
            const existingAdapter = adapterManager.getAdaptersByType('twitch')
            if (existingAdapter.length === 0) {
              try {
                await adapterManager.createAdapter({
                  id: 'twitch-adapter',
                  type: 'twitch',
                  name: 'Twitch Adapter',
                  enabled: true,
                  options: {
                    channelName: '',
                    includeSubscriptions: false,
                    eventsToTrack: [
                      'channel.update',
                      'stream.online',
                      'stream.offline',
                      'chat.message',
                      'chat.cheer',
                      'chat.subscription',
                      'chat.raid'
                    ]
                  }
                })

                logger.info('Twitch adapter created successfully')
              } catch (error) {
                logger.error('Failed to create Twitch adapter', {
                  error: error instanceof Error ? error.message : String(error)
                })
              }
            } else {
              logger.info('Twitch adapter already exists')
            }
          }
        }
      } catch (error) {
        logger.error('Error handling auth event', {
          error: error instanceof Error ? error.message : String(error)
        })
      }
    })

    // Store the subscription for cleanup
    subscriptionManager.add(authEventSubscription)

    // Set up tRPC server
    setupTRPCServer(mainEventBus, adapterManager, configStore, authService)

    logger.info('Services initialized successfully')
  } catch (error) {
    // Report error through error service if available
    if (errorService) {
      errorService.reportError(error instanceof Error ? error : new Error(String(error)), {
        component: 'ApplicationCore',
        operation: 'initializeServices',
        recoverable: false
      })
    } else {
      // Fallback if error service isn't initialized yet
      logger.error('Failed to initialize services', {
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined
      })
      mainEventBus?.publish(
        createSystemEvent(SystemEventType.ERROR, 'Failed to initialize services', 'error', {
          error: error instanceof Error ? error.message : String(error)
        })
      )
    }
  }
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.whenReady().then(async () => {
  // Set app user model id for windows
  electronApp.setAppUserModelId('com.electron')

  // Default open or close DevTools by F12 in development
  // and ignore CommandOrControl + R in production.
  // see https://github.com/alex8088/electron-toolkit/tree/master/packages/utils
  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  // Initialize services
  await initializeServices()

  // Create the main window
  createWindow()

  app.on('activate', function () {
    // On macOS it's common to re-create a window in the app when the
    // dock icon is clicked and there are no other windows open.
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

// Quit when all windows are closed, except on macOS. There, it's common
// for applications and their menu bar to stay active until the user quits
// explicitly with Cmd + Q.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// Handle app exit
app.on('before-quit', async (event) => {
  // If we're already cleaning up, don't prevent the quit
  if ((app as any).isCleaningUp) return

  // Prevent the app from quitting immediately
  event.preventDefault()

  // Set a flag to indicate we're cleaning up
  ;(app as any).isCleaningUp = true

  // Publish shutdown event
  mainEventBus?.publish(
    createSystemEvent(SystemEventType.SHUTDOWN, 'Zelan application shutting down', 'info')
  )

  try {
    // Clean up services
    if (adapterManager) {
      await adapterManager.dispose()
    }

    if (webSocketService) {
      webSocketService.stop()
    }

    if (authService) {
      authService.dispose()
    }

    // Clean up subscriptions
    subscriptionManager.unsubscribeAll()

    // Wait a moment for cleanup to complete and events to be processed
    setTimeout(() => {
      app.quit()
    }, 200)
  } catch (error) {
    // Report error through error service if available
    if (errorService) {
      errorService.reportError(error instanceof Error ? error : new Error(String(error)), {
        component: 'ApplicationCore',
        operation: 'appCleanup',
        recoverable: false
      })
    } else {
      logger.error('Error during cleanup', {
        error: error instanceof Error ? error.message : String(error)
      })
    }

    // Force quit if cleanup fails
    app.quit()
  } finally {
    // Clean up error service as the last step
    if (errorService) {
      errorService.dispose()
    }
  }
})

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.
