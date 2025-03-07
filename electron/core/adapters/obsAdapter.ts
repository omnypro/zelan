import { interval, Subscription, takeUntil, filter } from 'rxjs';
import OBSWebSocket from 'obs-websocket-js';
import { BaseAdapter } from './baseAdapter';
import { EventBus, createEvent } from '~/core/events';
import { AdapterSettingsStore } from '~/store';
import { 
  ObsAdapterConfig, 
  ObsAdapterConfigSchema, 
  AdapterState, 
  EventType, 
  BaseEventSchema
} from '@shared/types';

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
    // Try to get settings from AdapterSettingsStore
    let mergedConfig = config;
    
    try {
      const adapterSettingsStore = AdapterSettingsStore.getInstance();
      const savedSettings = adapterSettingsStore.getSettings('obs-adapter');
      
      if (savedSettings) {
        // Merge saved settings with provided config, with provided config taking precedence
        mergedConfig = {
          ...savedSettings,
          ...config
        };
        console.log('OBS adapter loaded settings from AdapterSettingsStore');
      }
    } catch (error) {
      console.warn('Could not load OBS adapter settings:', error);
    }
    
    super('obs-adapter', 'OBS Studio', ObsAdapterConfigSchema, mergedConfig);
  }
  
  /**
   * Implementation of connect
   */
  protected async connectImpl(): Promise<void> {
    // Connect to OBS
    const { host, port, password, secure } = this.config;
    const protocol = secure ? 'wss' : 'ws';
    const connectString = `${protocol}://${host}:${port}`;
    
    console.log(`Connecting to OBS at ${connectString}`);
    
    await this.obsClient.connect(connectString, password);
    
    // Register event handlers
    this.registerEventHandlers();
    
    // Get initial state
    await this.getInitialState();
    
    // Start status check timer
    this.startStatusCheckTimer();
  }
  
  /**
   * Implementation of disconnect
   */
  protected async disconnectImpl(): Promise<void> {
    // Stop timers
    this.stopStatusCheckTimer();
    this.stopReconnectTimer();
    
    // Disconnect from OBS
    await this.obsClient.disconnect();
  }
  
  /**
   * Get the current connection status
   */
  public async getConnectionStatus(): Promise<boolean> {
    try {
      // Check if OBS client is connected
      return this.obsClient.identified;
    } catch (error) {
      console.error('Error getting OBS connection status:', error);
      return false;
    }
  }
  
  /**
   * Get the current scene
   */
  public async getCurrentScene(): Promise<string | null> {
    if (!this.isConnected()) {
      return null;
    }
    
    try {
      const { currentProgramSceneName } = await this.obsClient.call('GetCurrentProgramScene');
      this.currentScene = currentProgramSceneName;
      return currentProgramSceneName;
    } catch (error) {
      console.error('Error getting current scene:', error);
      return null;
    }
  }
  
  /**
   * Get the current streaming status
   */
  public async getStreamingStatus(): Promise<{ streaming: boolean; recording: boolean; }> {
    if (!this.isConnected()) {
      return { streaming: false, recording: false };
    }
    
    try {
      const streamingStatus = await this.obsClient.call('GetStreamStatus');
      const recordingStatus = await this.obsClient.call('GetRecordStatus');
      
      this.streamingStatus = {
        streaming: streamingStatus.outputActive,
        recording: recordingStatus.outputActive,
      };
      
      return this.streamingStatus;
    } catch (error) {
      console.error('Error getting streaming status:', error);
      return { streaming: false, recording: false };
    }
  }
  
  /**
   * Register OBS event handlers
   */
  private registerEventHandlers(): void {
    // Scene changed
    this.obsClient.on('CurrentProgramSceneChanged', (data) => {
      const previousScene = this.currentScene;
      this.currentScene = data.sceneName;
      
      this.publishSceneChangedEvent(data.sceneName, previousScene);
    });
    
    // Streaming started
    this.obsClient.on('StreamStateChanged', (data) => {
      const nowStreaming = data.outputActive;
      const wasStreaming = this.streamingStatus.streaming;
      
      // Update status
      this.streamingStatus.streaming = nowStreaming;
      
      // Publish event on state change
      if (nowStreaming !== wasStreaming) {
        this.publishStreamingStatusEvent(nowStreaming);
      }
    });
    
    // Recording state changed
    this.obsClient.on('RecordStateChanged', (data) => {
      // Update status
      this.streamingStatus.recording = data.outputActive;
    });
    
    // Source visibility changed
    this.obsClient.on('SceneItemEnableStateChanged', (data) => {
      this.publishSourceVisibilityEvent(data.sceneName, data.sceneItemId, data.sceneItemEnabled);
    });
  }
  
  /**
   * Get initial OBS state
   */
  private async getInitialState(): Promise<void> {
    await this.getCurrentScene();
    await this.getStreamingStatus();
  }
  
  /**
   * Start the status check timer
   */
  private startStatusCheckTimer(): void {
    // Stop existing timer
    this.stopStatusCheckTimer();
    
    // Start new timer
    const statusCheckInterval = this.config.statusCheckInterval;
    
    this.statusCheckTimer = interval(statusCheckInterval)
      .pipe(
        takeUntil(this.destroy$),
        filter(() => this.isConnected())
      )
      .subscribe(async () => {
        try {
          const connected = await this.getConnectionStatus();
          
          if (!connected && this.isConnected()) {
            console.log('Status check detected OBS disconnection');
            this.setState(AdapterState.DISCONNECTED);
            this.publishDisconnectedEvent();
            this.startReconnectTimer();
          }
        } catch (error) {
          console.error('Error checking OBS status:', error);
        }
      });
  }
  
  /**
   * Stop the status check timer
   */
  private stopStatusCheckTimer(): void {
    if (this.statusCheckTimer) {
      this.statusCheckTimer.unsubscribe();
      this.statusCheckTimer = null;
    }
  }
  
  /**
   * Start the reconnect timer
   */
  private startReconnectTimer(): void {
    // Stop existing timer
    this.stopReconnectTimer();
    
    // Only start reconnect timer if autoConnect is enabled
    if (!this.config.autoConnect) {
      return;
    }
    
    // Start new timer
    const reconnectInterval = this.config.reconnectInterval;
    
    this.reconnectTimer = interval(reconnectInterval)
      .pipe(
        takeUntil(this.destroy$),
        filter(() => !this.isConnected())
      )
      .subscribe(async () => {
        console.log('Attempting to reconnect to OBS...');
        await this.connect();
      });
  }
  
  /**
   * Stop the reconnect timer
   */
  private stopReconnectTimer(): void {
    if (this.reconnectTimer) {
      this.reconnectTimer.unsubscribe();
      this.reconnectTimer = null;
    }
  }
  
  /**
   * Publish scene changed event
   */
  private publishSceneChangedEvent(sceneName: string, previousSceneName: string | null): void {
    const eventBus = EventBus.getInstance();
    
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: EventType.OBS_SCENE_CHANGED,
        source: 'obs-adapter',
        adapterId: this.adapterId,
        data: {
          sceneName,
          previousSceneName
        }
      }
    ));
  }
  
  /**
   * Publish streaming status event
   */
  private publishStreamingStatusEvent(streaming: boolean): void {
    const eventBus = EventBus.getInstance();
    
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: EventType.OBS_STREAMING_STATUS,
        source: 'obs-adapter',
        adapterId: this.adapterId,
        data: {
          streaming,
          recording: this.streamingStatus.recording
        }
      }
    ));
  }
  
  /**
   * Publish source visibility event
   */
  private publishSourceVisibilityEvent(sceneName: string, sceneItemId: number, visible: boolean): void {
    const eventBus = EventBus.getInstance();
    
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: EventType.OBS_SOURCE_VISIBILITY,
        source: 'obs-adapter',
        adapterId: this.adapterId,
        data: {
          sceneName,
          sceneItemId,
          visible
        }
      }
    ));
  }
}