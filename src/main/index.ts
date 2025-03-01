import { app, shell, BrowserWindow } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'

// Import event bus and system events
import { mainEventBus } from '@main/services/eventBus'
import { SystemStartupEvent, SystemShutdownEvent, SystemInfoEvent } from '@shared/core/events'

// Import adapter system
import { adapterManager } from '@main/services/adapters'
import { adapterFactories } from '@main/adapters'

// Track the main window instance
let mainWindow: BrowserWindow | null = null

/**
 * Initialize the event system
 */
function initializeEventSystem(): void {
  // Subscribe to events for system operations
  console.log('Initializing event system...')

  // Emit startup event
  const startupPayload = {
    appVersion: app.getVersion(),
    startTime: Date.now()
  }
  mainEventBus.publish(new SystemStartupEvent(startupPayload))

  // Log info event
  mainEventBus.publish(new SystemInfoEvent('Event system initialized'))
}

/**
 * Initialize the adapter system
 */
async function initializeAdapterSystem(): Promise<void> {
  console.log('Initializing adapter system...')
  
  // Register adapter factories
  adapterManager.registerFactories(adapterFactories)
  
  // Load saved adapters
  await adapterManager.loadAdapters()
  
  // Create a test adapter if none exists
  const testAdapters = adapterManager.getAdaptersByType('test')
  if (testAdapters.length === 0) {
    console.log('Creating default test adapter...')
    const adapter = await adapterManager.createAdapter('test', {
      name: 'Default Test Adapter',
      enabled: true,
      autoConnect: true,
      settings: {
        eventInterval: 10000, // 10 seconds
        eventTypes: ['info', 'warning'],
        simulateFailures: false,
        failureRate: 0.1
      }
    })
    
    mainEventBus.publish(new SystemInfoEvent(`Created default test adapter: ${adapter.name} (${adapter.id})`))
  }
  
  mainEventBus.publish(new SystemInfoEvent('Adapter system initialized'))
}

/**
 * Create the main application window
 */
function createWindow(): void {
  // Create the browser window.
  mainWindow = new BrowserWindow({
    width: 900,
    height: 670,
    show: false,
    autoHideMenuBar: true,
    ...(process.platform === 'linux' ? { icon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow?.show()

    // Emit event when window is shown
    mainEventBus.publish(new SystemInfoEvent('Main window ready'))
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

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.whenReady().then(() => {
  // Set app user model id for windows
  electronApp.setAppUserModelId('com.electron')

  // Default open or close DevTools by F12 in development
  // and ignore CommandOrControl + R in production.
  // see https://github.com/alex8088/electron-toolkit/tree/master/packages/utils
  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  // Initialize the event system
  initializeEventSystem()

  // Initialize the adapter system
  initializeAdapterSystem()
    .catch(error => {
      console.error('Failed to initialize adapter system:', error)
      mainEventBus.publish(new SystemInfoEvent(`Error initializing adapter system: ${error.message}`))
    })

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

// Handle application quit
app.on('before-quit', async (event) => {
  // Prevent the app from quitting immediately
  event.preventDefault()

  try {
    // Emit shutdown event
    mainEventBus.publish(new SystemShutdownEvent())

    // Dispose of adapters
    await adapterManager.dispose()
    
    // Now actually quit
    app.exit()
  } catch (error) {
    console.error('Error during shutdown:', error)
    app.exit(1) // Exit with error code
  }
})

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.
