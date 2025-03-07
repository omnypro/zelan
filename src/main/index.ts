import { app, shell, BrowserWindow, ipcMain } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'

// Import our services
import { MainEventBus } from './services/eventBus';
import { AdapterManager } from './services/adapters';
import { AdapterRegistry } from '../shared/adapters';
import { TestAdapterFactory } from './adapters';
import { createConfigStore, getConfigStore } from '../shared/core/config';
import { SystemStartupEvent } from '../shared/core/events';

// Global references
let mainWindow: BrowserWindow | null = null;
let mainEventBus: MainEventBus | null = null;
let adapterRegistry: AdapterRegistry | null = null;
let adapterManager: AdapterManager | null = null;

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
      mainEventBus.addWebContents(mainWindow.webContents);
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
 * Initialize the application core services
 */
async function initializeServices(): Promise<void> {
  try {
    // Create the configuration store
    const configStore = createConfigStore();
    
    // Initialize event bus
    mainEventBus = new MainEventBus();
    
    // Set up adapter registry
    adapterRegistry = new AdapterRegistry();
    
    // Register adapter factories
    adapterRegistry.register(new TestAdapterFactory());
    
    // Initialize adapter manager
    adapterManager = new AdapterManager(adapterRegistry, mainEventBus, configStore);
    await adapterManager.initialize();
    
    // Create a test adapter if none exists
    const adapters = adapterManager.getAllAdapters();
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
      });
    }
    
    // Publish startup event
    mainEventBus.publish(new SystemStartupEvent(app.getVersion()));
    
  } catch (error) {
    console.error('Failed to initialize services:', error);
  }
}

/**
 * Set up IPC handlers for configuration
 */
function setupConfigIpc(): void {
  const configStore = getConfigStore();
  const CONFIG_CHANGE_CHANNEL = 'zelan:config-change';
  
  // Subscribe to config changes and broadcast to renderer
  configStore.changes$().subscribe(changeEvent => {
    // Broadcast to all windows
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send(CONFIG_CHANGE_CHANNEL, changeEvent);
    }
  });
  
  // Get config value
  ipcMain.handle('config:get', (_, key, defaultValue) => {
    try {
      return configStore.get(key, defaultValue);
    } catch (error) {
      console.error(`Error getting config value for ${key}:`, error);
      return defaultValue;
    }
  });
  
  // Set config value
  ipcMain.handle('config:set', (_, key, value) => {
    try {
      configStore.set(key, value);
      return true;
    } catch (error) {
      console.error(`Error setting config value for ${key}:`, error);
      return false;
    }
  });
  
  // Update multiple config values
  ipcMain.handle('config:update', (_, updates) => {
    try {
      configStore.update(updates);
      return true;
    } catch (error) {
      console.error('Error updating multiple config values:', error);
      return false;
    }
  });
  
  // Get the config file path
  ipcMain.handle('config:path', () => {
    try {
      return configStore.fileName;
    } catch (error) {
      console.error('Error getting config file path:', error);
      return null;
    }
  });
  
  // Delete a config key
  ipcMain.handle('config:delete', (_, key) => {
    try {
      configStore.delete(key);
      return true;
    } catch (error) {
      console.error(`Error deleting config key ${key}:`, error);
      return false;
    }
  });
  
  // Check if a config key exists
  ipcMain.handle('config:has', (_, key) => {
    try {
      return configStore.has(key);
    } catch (error) {
      console.error(`Error checking config key ${key}:`, error);
      return false;
    }
  });
  
  // Get all config data
  ipcMain.handle('config:getAll', () => {
    try {
      return configStore.getAll();
    } catch (error) {
      console.error('Error getting all config data:', error);
      return null;
    }
  });
}

/**
 * Clean up application resources
 */
async function cleanupServices(): Promise<void> {
  try {
    // Dispose of adapters
    if (adapterManager) {
      await adapterManager.dispose();
    }
    
    // Clean up references
    mainEventBus = null;
    adapterRegistry = null;
    adapterManager = null;
  } catch (error) {
    console.error('Error during cleanup:', error);
  }
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.whenReady().then(async () => {
  try {
    // Set app user model id for windows
    electronApp.setAppUserModelId('com.electron')
  
    // Default open or close DevTools by F12 in development
    // and ignore CommandOrControl + R in production.
    // see https://github.com/alex8088/electron-toolkit/tree/master/packages/utils
    app.on('browser-window-created', (_, window) => {
      optimizer.watchWindowShortcuts(window)
    })
    
    // Initialize services
    await initializeServices();
    
    // Setup config IPC handlers
    setupConfigIpc();
    
    // IPC test
    ipcMain.on('ping', () => console.log('pong'))
  
    createWindow()
  
    app.on('activate', function () {
      // On macOS it's common to re-create a window in the app when the
      // dock icon is clicked and there are no other windows open.
      if (BrowserWindow.getAllWindows().length === 0) createWindow()
    })
  } catch (error) {
    console.error('Error during app initialization:', error);
  }
})

// Quit when all windows are closed, except on macOS. There, it's common
// for applications and their menu bar to stay active until the user quits
// explicitly with Cmd + Q.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// Clean up services when quitting
app.on('will-quit', async (event) => {
  event.preventDefault();
  await cleanupServices();
  app.exit();
})