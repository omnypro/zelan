import { z } from 'zod';
import { interval, Subscription, takeUntil, filter } from 'rxjs';
import OBSWebSocket from 'obs-websocket-js';
import { BaseAdapter } from './baseAdapter';
import { EventBus, EventType, createEvent, BaseEventSchema } from '../events';
import { AdapterConfig, AdapterState } from './types';

/**
 * OBS event schemas for specific OBS events
 */
export const ObsSceneChangedEventSchema = BaseEventSchema.extend({
  sceneName: z.string(),
  previousSceneName: z.string().optional(),
});

export const ObsStreamingStatusEventSchema = BaseEventSchema.extend({
  streaming: z.boolean(),
  recording: z.boolean(),
  replayBufferActive: z.boolean().optional(),
  bytesPerSec: z.number().optional(),
  kbitsPerSec: z.number().optional(),
  duration: z.number().optional(),
});

export const ObsSourceVisibilityEventSchema = BaseEventSchema.extend({
  sourceName: z.string(),
  visible: z.boolean(),
  sceneItemId: z.number().optional(),
  sceneName: z.string().optional(),
});

export type ObsSceneChangedEvent = z.infer<typeof ObsSceneChangedEventSchema>;
export type ObsStreamingStatusEvent = z.infer<typeof ObsStreamingStatusEventSchema>;
export type ObsSourceVisibilityEvent = z.infer<typeof ObsSourceVisibilityEventSchema>;

/**
 * OBS adapter configuration schema
 */
export const ObsAdapterConfigSchema = z.object({
  enabled: z.boolean().default(true),
  name: z.string().optional(),
  autoConnect: z.boolean().default(true),
  address: z.string().default('localhost'), 
  port: z.number().default(4455),
  password: z.string().optional(),
  reconnectInterval: z.number().min(1000).default(5000),
  statusCheckInterval: z.number().min(1000).default(10000),
});

export type ObsAdapterConfig = z.infer<typeof ObsAdapterConfigSchema>;

/**
 * Defines all the OBS event types that we care about
 */
export enum ObsEventType {
  SCENE_CHANGED = 'obs.scene.changed',
  STREAMING_STARTED = 'obs.streaming.started',
  STREAMING_STOPPED = 'obs.streaming.stopped',
  RECORDING_STARTED = 'obs.recording.started',
  RECORDING_STOPPED = 'obs.recording.stopped',
  SOURCE_ACTIVATED = 'obs.source.activated',
  SOURCE_DEACTIVATED = 'obs.source.deactivated',
  CONNECTION_STATUS = 'obs.connection.status',
}

/**
 * OBS adapter implementation to connect to OBS Studio
 */
export class ObsAdapter extends BaseAdapter<ObsAdapterConfig> {
  private obsClient: OBSWebSocket = new OBSWebSocket();
  private statusCheckTimer: Subscription | null = null;
  private reconnectTimer: Subscription | null = null;
  private currentScene: string | null = null;
  private streamingStatus = {
    streaming: false,
    recording: false,
  };
  
  /**
   * Create a new OBS adapter instance
   */
  constructor(config: Partial<ObsAdapterConfig> = {}) {
    // Try to get settings from AdapterSettingsManager
    let mergedConfig = config;
    
    try {
      const { AdapterSettingsManager } = require('../config/adapterSettingsManager');
      const settingsManager = AdapterSettingsManager.getInstance();
      const savedSettings = settingsManager.getSettings('obs-adapter');
      
      if (savedSettings) {
        // Map the host field from settings to address field for the adapter
        const mappedSettings: Partial<ObsAdapterConfig> = {
          ...savedSettings,
          // Map 'host' to 'address' if it exists
          address: savedSettings.host || savedSettings.address || 'localhost',
        };
        
        // Remove host if it exists to avoid confusing the adapter
        if ('host' in mappedSettings) {
          delete mappedSettings.host;
        }
        
        // Merge mapped settings with provided config, with provided config taking precedence
        mergedConfig = {
          ...mappedSettings,
          ...config
        };
        
        console.log('OBS adapter loaded settings from AdapterSettingsManager');
      }
    } catch (error) {
      console.log('Using default OBS adapter settings');
    }
    
    super(
      'obs-adapter',
      'OBS Studio',
      mergedConfig,
      ObsAdapterConfigSchema
    );
  }
  
  /**
   * Connect to OBS WebSocket server
   */
  protected async connectImpl(): Promise<void> {
    try {
      // Stop any existing reconnect timer
      this.stopReconnectTimer();
      
      // Connect to OBS
      const { address, port, password } = this.config;
      const connectionParams: { address: string; password?: string } = {
        address: `ws://${address}:${port}`
      };
      
      if (password) {
        connectionParams.password = password;
      }
      
      console.log(`Connecting to OBS at ${connectionParams.address}`);
      await this.obsClient.connect(connectionParams.address, connectionParams.password);
      console.log('Connected to OBS Studio');
      
      // Get current scene
      const sceneData = await this.obsClient.call('GetCurrentProgramScene');
      this.currentScene = sceneData.currentProgramSceneName;
      
      // Get current streaming status
      const streamStatus = await this.obsClient.call('GetStreamStatus');
      this.streamingStatus.streaming = streamStatus.outputActive;
      
      // Get current recording status
      const recordStatus = await this.obsClient.call('GetRecordStatus');
      this.streamingStatus.recording = recordStatus.outputActive;
      
      // Register for events
      this.registerEventHandlers();
      
      // Start status check timer
      this.startStatusCheckTimer();
      
      // Publish connection status event
      this.publishConnectionStatusEvent(true);
    } catch (error) {
      console.error('Failed to connect to OBS:', error);
      this.startReconnectTimer();
      throw error;
    }
  }
  
  /**
   * Disconnect from OBS WebSocket server
   */
  protected async disconnectImpl(): Promise<void> {
    try {
      // Stop timers
      this.stopStatusCheckTimer();
      this.stopReconnectTimer();
      
      // Check if we're actually connected before trying to disconnect
      if (this.obsClient && this.obsClient.identified) {
        // Disconnect from OBS
        await this.obsClient.disconnect();
      }
      
      // Always update state and publish event, even if disconnect throws
      this.updateState(AdapterState.DISCONNECTED);
      
      // Publish connection status event
      this.publishConnectionStatusEvent(false);
      console.log('Disconnected from OBS Studio');
    } catch (error) {
      console.error('Error disconnecting from OBS:', error);
      // Still mark as disconnected even if there was an error
      this.updateState(AdapterState.DISCONNECTED);
      this.publishConnectionStatusEvent(false, error as Error);
      // Don't throw, just log the error
    }
  }
  
  /**
   * Clean up resources
   */
  protected destroyImpl(): void {
    this.stopStatusCheckTimer();
    this.stopReconnectTimer();
    
    // Ensure OBS client is disconnected
    try {
      // Only try to disconnect if we have a client and it's identified
      if (this.obsClient && this.obsClient.identified) {
        this.obsClient.disconnect();
      }
    } catch (e) {
      // Ignore errors during destroy
      console.log('Non-critical error during OBS adapter cleanup:', e);
    }
    
    // Attempt to remove all event listeners
    try {
      if (this.obsClient) {
        this.obsClient.removeAllListeners();
      }
    } catch (e) {
      // Ignore errors
    }
  }
  
  /**
   * Register event handlers for OBS events
   */
  private registerEventHandlers(): void {
    // Program scene changed event
    this.obsClient.on('CurrentProgramSceneChanged', (data) => {
      const prevScene = this.currentScene;
      this.currentScene = data.sceneName;
      
      this.publishSceneChangedEvent(data.sceneName, prevScene || undefined);
    });
    
    // Stream status changed
    this.obsClient.on('StreamStateChanged', (data) => {
      const wasStreaming = this.streamingStatus.streaming;
      this.streamingStatus.streaming = data.outputActive;
      
      if (wasStreaming !== data.outputActive) {
        if (data.outputActive) {
          this.publishStreamingStartedEvent();
        } else {
          this.publishStreamingStoppedEvent();
        }
      }
    });
    
    // Recording status changed
    this.obsClient.on('RecordStateChanged', (data) => {
      const wasRecording = this.streamingStatus.recording;
      this.streamingStatus.recording = data.outputActive;
      
      if (wasRecording !== data.outputActive) {
        if (data.outputActive) {
          this.publishRecordingStartedEvent();
        } else {
          this.publishRecordingStoppedEvent();
        }
      }
    });
    
    // Scene item visibility (source) changed
    this.obsClient.on('SceneItemEnableStateChanged', (data) => {
      this.publishSourceVisibilityEvent(
        data.sceneName,
        data.sceneItemId,
        data.sceneItemEnabled
      );
    });
    
    // Handle disconnection
    this.obsClient.on('ConnectionClosed', () => {
      if (this.isConnected()) {
        console.warn('OBS WebSocket connection closed unexpectedly, attempting to reconnect...');
        this.updateState(AdapterState.DISCONNECTED);
        this.publishConnectionStatusEvent(false);
        this.startReconnectTimer();
      }
    });
    
    // Handle error
    this.obsClient.on('ConnectionError', (error) => {
      console.error('OBS WebSocket connection error:', error);
      this.publishConnectionStatusEvent(false, error);
    });
  }
  
  /**
   * Start periodic status check
   */
  private startStatusCheckTimer(): void {
    // Clear any existing timer
    this.stopStatusCheckTimer();
    
    // Create new timer
    this.statusCheckTimer = interval(this.config.statusCheckInterval)
      .pipe(
        takeUntil(this.destroyed$),
        filter(() => this.isConnected())
      )
      .subscribe(async () => {
        try {
          // Check if OBS is still connected
          if (!this.obsClient.identified) {
            throw new Error('OBS WebSocket not identified');
          }
          
          // Get stream status
          const streamStatus = await this.obsClient.call('GetStreamStatus');
          
          // Get recording status
          const recordStatus = await this.obsClient.call('GetRecordStatus');
          
          // Update local state
          this.streamingStatus.streaming = streamStatus.outputActive;
          this.streamingStatus.recording = recordStatus.outputActive;
          
          // Publish status event with detailed information
          this.publishStreamingStatusEvent(
            streamStatus.outputActive,
            recordStatus.outputActive,
            false,
            streamStatus.outputBytes,
            streamStatus.outputDuration
          );
        } catch (error) {
          console.error('Error during OBS status check:', error);
          
          // If we can't connect, start reconnect process
          if (this.isConnected()) {
            console.warn('OBS status check failed, disconnecting...');
            this.updateState(AdapterState.DISCONNECTED);
            this.publishConnectionStatusEvent(false);
            
            // Ensure we're actually disconnected from OBS
            try {
              this.obsClient.disconnect();
            } catch (disconnectError) {
              // Ignore any errors during forced disconnect
            }
            
            this.startReconnectTimer();
          }
        }
      });
  }
  
  /**
   * Stop status check timer
   */
  private stopStatusCheckTimer(): void {
    if (this.statusCheckTimer) {
      this.statusCheckTimer.unsubscribe();
      this.statusCheckTimer = null;
    }
  }
  
  /**
   * Start reconnect timer
   */
  private startReconnectTimer(): void {
    // Clear any existing timer
    this.stopReconnectTimer();
    
    // Create new timer
    this.reconnectTimer = interval(this.config.reconnectInterval)
      .pipe(
        takeUntil(this.destroyed$),
        filter(() => !this.isConnected() && this.config.enabled)
      )
      .subscribe(() => {
        console.log('Attempting to reconnect to OBS...');
        this.connect().catch((err) => {
          console.log('OBS reconnect failed:', err.message);
        });
      });
  }
  
  /**
   * Stop reconnect timer
   */
  private stopReconnectTimer(): void {
    if (this.reconnectTimer) {
      this.reconnectTimer.unsubscribe();
      this.reconnectTimer = null;
    }
  }
  
  /**
   * Handle configuration changes
   */
  protected override handleConfigChange(): void {
    // Save updated config to AdapterSettingsManager
    try {
      const { AdapterSettingsManager } = require('../config/adapterSettingsManager');
      const settingsManager = AdapterSettingsManager.getInstance();
      
      // Map address to host for storing in settings manager
      const settingsToSave = {
        ...this.config,
        host: this.config.address, // Map address back to host for storage
      };
      
      // Remove address to avoid duplicate storage
      delete settingsToSave.address;
      
      settingsManager.updateSettings('obs-adapter', settingsToSave);
      console.log('OBS adapter settings saved to AdapterSettingsManager');
    } catch (error) {
      console.error('Failed to save OBS adapter settings:', error);
    }
    
    // If connection params changed and we're connected, reconnect
    if (this.isConnected()) {
      this.disconnect()
        .then(() => {
          if (this.config.enabled) {
            return this.connect();
          }
        })
        .catch(console.error);
    } else {
      // Call the parent implementation for connect/disconnect handling
      super.handleConfigChange();
    }
  }
  
  // Event publication methods
  
  /**
   * Publish scene changed event
   */
  private publishSceneChangedEvent(sceneName: string, previousSceneName?: string): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      ObsSceneChangedEventSchema,
      {
        type: ObsEventType.SCENE_CHANGED,
        source: this.adapterId,
        sceneName,
        previousSceneName
      }
    );
    
    eventBus.publish(event);
  }
  
  /**
   * Publish streaming started event
   */
  private publishStreamingStartedEvent(): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      BaseEventSchema,
      {
        type: ObsEventType.STREAMING_STARTED,
        source: this.adapterId
      }
    );
    
    eventBus.publish(event);
  }
  
  /**
   * Publish streaming stopped event
   */
  private publishStreamingStoppedEvent(): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      BaseEventSchema,
      {
        type: ObsEventType.STREAMING_STOPPED,
        source: this.adapterId
      }
    );
    
    eventBus.publish(event);
  }
  
  /**
   * Publish recording started event
   */
  private publishRecordingStartedEvent(): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      BaseEventSchema,
      {
        type: ObsEventType.RECORDING_STARTED,
        source: this.adapterId
      }
    );
    
    eventBus.publish(event);
  }
  
  /**
   * Publish recording stopped event
   */
  private publishRecordingStoppedEvent(): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      BaseEventSchema,
      {
        type: ObsEventType.RECORDING_STOPPED,
        source: this.adapterId
      }
    );
    
    eventBus.publish(event);
  }
  
  /**
   * Publish detailed streaming status event
   */
  private publishStreamingStatusEvent(
    streaming: boolean,
    recording: boolean,
    replayBufferActive?: boolean,
    bytesPerSec?: number,
    duration?: number
  ): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      ObsStreamingStatusEventSchema,
      {
        type: 'obs.streaming.status',
        source: this.adapterId,
        streaming,
        recording,
        replayBufferActive,
        bytesPerSec: bytesPerSec || 0,
        kbitsPerSec: bytesPerSec ? Math.round(bytesPerSec / 125) : 0, // 1000 bits / 8 = 125 bytes
        duration: duration || 0
      }
    );
    
    eventBus.publish(event);
  }
  
  /**
   * Publish source visibility changed event
   */
  private publishSourceVisibilityEvent(
    sceneName: string,
    sceneItemId: number,
    visible: boolean
  ): void {
    const eventBus = EventBus.getInstance();
    
    // We need to get the source name from the sceneItemId
    this.obsClient.call('GetSceneItemList', { sceneName })
      .then(data => {
        const sceneItem = data.sceneItems.find(item => item.sceneItemId === sceneItemId);
        if (sceneItem) {
          const event = createEvent(
            ObsSourceVisibilityEventSchema,
            {
              type: visible ? ObsEventType.SOURCE_ACTIVATED : ObsEventType.SOURCE_DEACTIVATED,
              source: this.adapterId,
              sourceName: sceneItem.sourceName,
              visible,
              sceneItemId,
              sceneName
            }
          );
          
          eventBus.publish(event);
        }
      })
      .catch(err => {
        console.error('Failed to get source name for visibility event:', err);
      });
  }
  
  /**
   * Publish connection status event
   */
  private publishConnectionStatusEvent(connected: boolean, error?: Error): void {
    const eventBus = EventBus.getInstance();
    
    const event = createEvent(
      BaseEventSchema,
      {
        type: ObsEventType.CONNECTION_STATUS,
        source: this.adapterId,
        data: {
          connected,
          error: error ? error.message : undefined
        }
      }
    );
    
    eventBus.publish(event);
  }
}