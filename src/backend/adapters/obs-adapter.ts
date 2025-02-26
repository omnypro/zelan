/**
 * OBS WebSocket adapter for Zelan
 * Connects to OBS Studio via WebSockets
 */

import OBSWebSocket from 'obs-websocket-js';
import { BaseAdapter, ServiceAdapter } from './base-adapter';
import { EventBus } from '../event-bus';
import { getEventBus, registerAdapter } from '../index';
import { createAdapterSettings, useZelanStore } from '../state';
import { ServiceStatus } from '../types';

/**
 * Configuration for the OBS adapter
 */
export interface OBSAdapterConfig {
  /** Host address of OBS WebSocket server */
  host: string;
  /** Port number of OBS WebSocket server */
  port: number;
  /** Password for OBS WebSocket server (if required) */
  password: string;
  /** Automatically reconnect on disconnect */
  autoReconnect: boolean;
  /** Include detailed scene information in events */
  includeSceneDetails: boolean;
}

// Default configuration
const DEFAULT_CONFIG: OBSAdapterConfig = {
  host: 'localhost',
  port: 4455,
  password: '',
  autoReconnect: true,
  includeSceneDetails: true
};

// Singleton instance
let obsAdapterInstance: OBSAdapter | null = null;

/**
 * Get the OBS adapter singleton instance
 */
export function getOBSAdapter(): OBSAdapter {
  if (!obsAdapterInstance) {
    const eventBus = getEventBus();
    obsAdapterInstance = new OBSAdapter(eventBus);
  }
  return obsAdapterInstance;
}

/**
 * Implementation of OBS Studio integration
 */
export class OBSAdapter implements ServiceAdapter {
  private base: BaseAdapter;
  private config: OBSAdapterConfig;
  private obs: OBSWebSocket;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private reconnectCount: number = 0;
  private sceneInfo: any = null;
  private streamStatus: any = null;
  
  /**
   * Create a new OBS adapter
   * @param eventBus EventBus instance
   * @param config Optional configuration
   */
  constructor(eventBus: EventBus, config?: Partial<OBSAdapterConfig>) {
    console.log('Creating new OBS adapter');
    this.base = new BaseAdapter('obs', eventBus);
    this.obs = new OBSWebSocket();
    
    // Apply default config values
    this.config = {
      ...DEFAULT_CONFIG,
      ...config
    };
    
    // Set up event handlers
    this.setupEventHandlers();
    
    // Register adapter in store
    const store = useZelanStore.getState();
    const settings = createAdapterSettings(
      'obs',
      true,
      this.config,
      'OBS Studio',
      'Connects to OBS Studio via WebSockets for scene changes, recording status, and other events'
    );
    store.registerAdapter('obs', settings);
  }
  
  /**
   * Get the adapter name
   */
  getName(): string {
    return this.base.getName();
  }
  
  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.base.isConnected();
  }
  
  /**
   * Configure the OBS adapter
   * @param config New configuration settings
   */
  async configure(config: Partial<OBSAdapterConfig>): Promise<void> {
    console.log('Configuring OBS adapter', config);
    
    // Merge configs
    this.config = {
      ...this.config,
      ...config
    };
    
    // Get current state
    const wasConnected = this.isConnected();
    
    // Disconnect if connected
    if (wasConnected) {
      await this.disconnect();
    }
    
    // Update state
    const store = useZelanStore.getState();
    const currentSettings = store.adapterSettings['obs'] || 
      createAdapterSettings('obs', true, this.config);
    
    // Create updated settings
    const newSettings = {
      ...currentSettings,
      config: this.config
    };
    
    // Update settings in store
    store.updateAdapterSettings('obs', newSettings);
    
    // Reconnect if needed with new config
    if (wasConnected) {
      await this.connect();
    }
  }
  
  /**
   * Connect to OBS Studio
   */
  async connect(): Promise<void> {
    // Only connect if not already connected
    if (this.isConnected()) {
      console.log('OBS adapter is already connected');
      return;
    }
    
    // Update status in store
    const store = useZelanStore.getState();
    store.updateAdapterStatus('obs', ServiceStatus.Connecting);
    
    // Clear any reconnect timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    
    try {
      console.log(`Connecting to OBS at ${this.config.host}:${this.config.port}`);
      
      // Create connection parameters
      const connectionParams: any = {
        address: `${this.config.host}:${this.config.port}`
      };
      
      // Add password if provided
      if (this.config.password) {
        connectionParams.password = this.config.password;
      }
      
      // Connect to OBS
      await this.obs.connect(connectionParams);
      
      // Set connected state
      this.base.setConnected(true);
      
      // Reset reconnect counter
      this.reconnectCount = 0;
      
      // Fetch initial data
      await this.fetchInitialData();
      
      // Update status in store
      store.updateAdapterStatus('obs', ServiceStatus.Connected);
      
      console.log('OBS adapter connected successfully');
    } catch (error) {
      console.error('Failed to connect to OBS:', error);
      
      // Set error status
      store.updateAdapterStatus('obs', ServiceStatus.Error);
      
      // Setup reconnect if enabled
      if (this.config.autoReconnect) {
        this.scheduleReconnect();
      }
    }
  }
  
  /**
   * Disconnect from OBS Studio
   */
  async disconnect(): Promise<void> {
    // Only disconnect if connected
    if (!this.isConnected()) {
      console.log('OBS adapter is already disconnected');
      return;
    }
    
    // Clear any reconnect timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    
    console.log('Disconnecting from OBS');
    
    // Set disconnected state
    this.base.setConnected(false);
    
    // Update status in store
    const store = useZelanStore.getState();
    store.updateAdapterStatus('obs', ServiceStatus.Disconnected);
    
    // Disconnect from OBS (don't await)
    try {
      await this.obs.disconnect();
    } catch (error) {
      console.error('Error during OBS disconnect:', error);
    }
    
    console.log('OBS adapter disconnected');
  }
  
  /**
   * Set up event handlers for OBS events
   */
  private setupEventHandlers(): void {
    // Handle connection and disconnection events
    this.obs.on('ConnectionOpened', () => {
      console.log('OBS connection opened');
    });
    
    this.obs.on('ConnectionClosed', () => {
      console.log('OBS connection closed');
      
      // Only handle as error if we think we're connected
      if (this.isConnected()) {
        console.log('Unexpected OBS disconnection');
        
        // Set disconnected state
        this.base.setConnected(false);
        
        // Update status in store
        const store = useZelanStore.getState();
        store.updateAdapterStatus('obs', ServiceStatus.Error);
        
        // Try to reconnect if enabled
        if (this.config.autoReconnect) {
          this.scheduleReconnect();
        }
      }
    });
    
    this.obs.on('ConnectionError', (err) => {
      console.error('OBS connection error:', err);
      
      // Set disconnected state
      this.base.setConnected(false);
      
      // Update status in store
      const store = useZelanStore.getState();
      store.updateAdapterStatus('obs', ServiceStatus.Error);
      
      // Try to reconnect if enabled
      if (this.config.autoReconnect) {
        this.scheduleReconnect();
      }
    });
    
    // Handle scene events
    this.obs.on('CurrentProgramSceneChanged', async (data) => {
      console.log('OBS scene changed:', data);
      
      // Get additional scene info if enabled
      let sceneDetails = null;
      if (this.config.includeSceneDetails) {
        try {
          // Get more details about the scene
          sceneDetails = await this.obs.call('GetSceneItemList', {
            sceneName: data.sceneName
          });
        } catch (error) {
          console.error('Failed to get scene details:', error);
        }
      }
      
      // Create event payload
      const payload = {
        sceneName: data.sceneName,
        previousSceneName: data.sceneName, // Not available in newer versions
        timestamp: new Date().toISOString(),
        details: sceneDetails
      };
      
      // Publish event
      this.base.publishEvent('scene.changed', payload);
    });
    
    // Handle recording events
    this.obs.on('RecordStateChanged', (data) => {
      console.log('OBS recording state changed:', data);
      
      // Create event payload
      const payload = {
        isRecording: data.outputActive,
        state: data.outputState,
        timestamp: new Date().toISOString()
      };
      
      // Publish event
      this.base.publishEvent('recording.state_changed', payload);
    });
    
    // Handle streaming events
    this.obs.on('StreamStateChanged', (data) => {
      console.log('OBS streaming state changed:', data);
      
      // Create event payload
      const payload = {
        isStreaming: data.outputActive,
        state: data.outputState,
        timestamp: new Date().toISOString()
      };
      
      // Publish event
      this.base.publishEvent('streaming.state_changed', payload);
      
      // Store stream status
      this.streamStatus = {
        active: data.outputActive,
        state: data.outputState,
        timestamp: new Date().toISOString()
      };
    });
  }
  
  /**
   * Fetch initial data from OBS
   */
  private async fetchInitialData(): Promise<void> {
    try {
      // Get current scene
      const sceneInfo = await this.obs.call('GetCurrentProgramScene');
      this.sceneInfo = sceneInfo;
      
      // Get streaming status
      const streamStatus = await this.obs.call('GetStreamStatus');
      this.streamStatus = streamStatus;
      
      // Publish initial scene event
      const scenePayload = {
        sceneName: sceneInfo.currentProgramSceneName,
        timestamp: new Date().toISOString()
      };
      this.base.publishEvent('scene.initial', scenePayload);
      
      // Publish initial stream status
      const streamPayload = {
        isStreaming: streamStatus.outputActive,
        streamTimecode: streamStatus.outputTimecode,
        timestamp: new Date().toISOString()
      };
      this.base.publishEvent('streaming.initial', streamPayload);
      
    } catch (error) {
      console.error('Failed to fetch initial OBS data:', error);
    }
  }
  
  /**
   * Schedule a reconnect attempt
   */
  private scheduleReconnect(): void {
    // Clear any existing reconnect timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }
    
    // Calculate delay with exponential backoff
    const delay = Math.min(
      30000, // Max 30 seconds
      1000 * Math.pow(1.5, Math.min(this.reconnectCount, 10)) // Exponential backoff
    );
    
    console.log(`Scheduling OBS reconnect in ${delay}ms (attempt ${this.reconnectCount + 1})`);
    
    // Set reconnect timer
    this.reconnectTimer = setTimeout(() => {
      this.reconnectCount++;
      this.connect();
    }, delay);
  }
}
