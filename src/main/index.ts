import { app, shell, BrowserWindow, ipcMain } from 'electron';
import { join } from 'path';
import { electronApp, optimizer, is } from '@electron-toolkit/utils';
import icon from '../../resources/icon.png?asset';

// Import our services
import { MainEventBus } from './services/eventBus';
import { AdapterManager } from './services/adapters';
import { WebSocketService } from './services/websocket';
import { AdapterRegistry } from '../shared/adapters';
import { EventCache } from './services/events/EventCache';
import { ConfigStore, getConfigStore } from '../shared/core/config';
import { TestAdapterFactory } from './adapters/test';
import { ObsAdapterFactory } from './adapters/obs';
import { setupTRPCServer } from './trpc';
import { SystemEventType } from '../shared/types/events';
import { createSystemEvent } from '../shared/core/events';

// Global references
let mainWindow: BrowserWindow | null = null;
let configStore: ConfigStore | null = null;
let eventCache: EventCache | null = null;
let mainEventBus: MainEventBus | null = null;
let adapterRegistry: AdapterRegistry | null = null;
let adapterManager: AdapterManager | null = null;
let webSocketService: WebSocketService | null = null;

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
  });

  mainWindow.on('ready-to-show', () => {
    mainWindow!.show();
    
    // Connect event bus to this window
    if (mainEventBus && mainWindow) {
      mainEventBus.addWebContents(mainWindow.webContents);
    }
  });

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url);
    return { action: 'deny' };
  });

  // HMR for renderer base on electron-vite cli.
  // Load the remote URL for development or the local html file for production.
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL']);
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'));
  }
}

/**
 * Set up IPC handlers for the config store
 */
function setupConfigIpcHandlers(config: ConfigStore): void {
  // Handler for getting config values
  ipcMain.handle('config:get', async (_, key: string, defaultValue?: unknown) => {
    return config.get(key, defaultValue);
  });

  // Handler for setting config values
  ipcMain.handle('config:set', async (_, key: string, value: unknown) => {
    config.set(key, value);
    return true;
  });

  // Handler for checking if a key exists
  ipcMain.handle('config:has', async (_, key: string) => {
    return config.has(key);
  });

  // Handler for deleting a key
  ipcMain.handle('config:delete', async (_, key: string) => {
    config.delete(key);
    return true;
  });

  // Handler for getting all config data
  ipcMain.handle('config:getAll', async () => {
    return config.getAll();
  });

  // Handler for updating multiple values
  ipcMain.handle('config:update', async (_, updates: Record<string, unknown>) => {
    config.update(updates);
    return true;
  });

  // Handler for getting the config file path
  ipcMain.handle('config:path', async () => {
    return config.fileName;
  });
  
  // Set up event forwarding for config changes
  const CONFIG_CHANGE_CHANNEL = 'zelan:config-change';
  
  // Subscribe to config changes and forward to renderer
  config.changes$().subscribe(event => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send(CONFIG_CHANGE_CHANNEL, event);
    }
  });
}

/**
 * Initialize the application core services
 */
async function initializeServices(): Promise<void> {
  try {
    // Create the configuration store
    configStore = getConfigStore();
    
    // Setup IPC handlers for config
    setupConfigIpcHandlers(configStore);
    
    // Create event cache
    eventCache = new EventCache(configStore);
    
    // Initialize event bus
    mainEventBus = new MainEventBus(eventCache);
    
    // Publish startup event
    mainEventBus.publish(
      createSystemEvent(
        SystemEventType.STARTUP,
        'Zelan application starting',
        'info',
        { version: app.getVersion() }
      )
    );
    
    // Set up adapter registry
    adapterRegistry = new AdapterRegistry();
    
    // Register adapter factories
    adapterRegistry.register(new TestAdapterFactory());
    adapterRegistry.register(new ObsAdapterFactory());
    
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
    
    // Initialize WebSocket service
    webSocketService = WebSocketService.getInstance(mainEventBus, configStore);
    
    // Get WebSocket settings from config or use defaults
    const wsSettings = configStore.getSettings();
    
    // Start WebSocket server automatically
    if (webSocketService) {
      webSocketService.start();
    }
    
    // Set up tRPC server
    setupTRPCServer(mainEventBus, adapterManager, configStore);
    
    console.log('Services initialized successfully');
  } catch (error) {
    console.error('Failed to initialize services:', error);
    mainEventBus?.publish(
      createSystemEvent(
        SystemEventType.ERROR,
        'Failed to initialize services',
        'error',
        { error: error instanceof Error ? error.message : String(error) }
      )
    );
  }
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.whenReady().then(async () => {
  // Set app user model id for windows
  electronApp.setAppUserModelId('com.electron');

  // Default open or close DevTools by F12 in development
  // and ignore CommandOrControl + R in production.
  // see https://github.com/alex8088/electron-toolkit/tree/master/packages/utils
  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window);
  });

  // Initialize services
  await initializeServices();
  
  // Create the main window
  createWindow();

  app.on('activate', function () {
    // On macOS it's common to re-create a window in the app when the
    // dock icon is clicked and there are no other windows open.
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

// Quit when all windows are closed, except on macOS. There, it's common
// for applications and their menu bar to stay active until the user quits
// explicitly with Cmd + Q.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// Handle app exit
app.on('before-quit', async (event) => {
  // If we're already cleaning up, don't prevent the quit
  if ((app as any).isCleaningUp) return;
  
  // Prevent the app from quitting immediately
  event.preventDefault();
  
  // Set a flag to indicate we're cleaning up
  (app as any).isCleaningUp = true;
  
  // Publish shutdown event
  mainEventBus?.publish(
    createSystemEvent(
      SystemEventType.SHUTDOWN,
      'Zelan application shutting down',
      'info'
    )
  );
  
  try {
    // Clean up services
    if (adapterManager) {
      await adapterManager.dispose();
    }
    
    if (webSocketService) {
      webSocketService.stop();
    }
    
    // Wait a moment for cleanup to complete and events to be processed
    setTimeout(() => {
      app.quit();
    }, 200);
  } catch (error) {
    console.error('Error during cleanup:', error);
    app.quit(); // Force quit if cleanup fails
  }
});

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.