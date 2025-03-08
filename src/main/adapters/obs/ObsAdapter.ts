import OBSWebSocket from 'obs-websocket-js';
import { EventEmitter } from 'events';
import { BaseAdapter } from '../../../shared/adapters/base';
import { EventBus } from '../../../shared/core/bus';
import { EventCategory } from '../../../shared/types/events';
import { 
  createObsEvent,
  ObsEventType
} from '../../../shared/core/events';
import { AdapterStatus } from '../../../shared/adapters/interfaces/AdapterStatus';

/**
 * OBS adapter options
 */
export interface ObsAdapterOptions {
  host: string;
  port: number;
  password?: string;
  reconnectInterval: number;
  autoReconnect: boolean;
}

/**
 * Default options for the OBS adapter
 */
const DEFAULT_OPTIONS: ObsAdapterOptions = {
  host: 'localhost',
  port: 4455,
  reconnectInterval: 5000,
  autoReconnect: true
};

/**
 * OBS adapter specific event type mapping
 * This maps internal event types to the standard ObsEventType enum
 */
const OBS_EVENT_TYPE_MAP: Record<string, ObsEventType> = {
  scene_switched: ObsEventType.SCENE_CHANGED,
  streaming_started: ObsEventType.STREAM_STARTED,
  streaming_stopped: ObsEventType.STREAM_STOPPED,
  recording_started: ObsEventType.RECORDING_STARTED,
  recording_stopped: ObsEventType.RECORDING_STOPPED,
  source_changed: ObsEventType.SOURCE_VISIBILITY_CHANGED,
  scene_collection_changed: ObsEventType.SCENE_COLLECTION_CHANGED,
  scene_list_changed: ObsEventType.SCENE_COLLECTION_CHANGED,
  virtual_cam_started: ObsEventType.VIRTUAL_CAM_STARTED,
  virtual_cam_stopped: ObsEventType.VIRTUAL_CAM_STOPPED
};

/**
 * OBS adapter for connecting to OBS Studio via websocket
 */
export class ObsAdapter extends BaseAdapter {
  private obs: OBSWebSocket;
  private reconnectTimer?: NodeJS.Timeout;
  private eventEmitter: EventEmitter;
  private isStreaming: boolean = false;
  private isRecording: boolean = false;
  private isVirtualCamActive: boolean = false;
  private currentScene: string = '';
  private scenes: string[] = [];

  constructor(
    id: string,
    name: string,
    options: Partial<ObsAdapterOptions>,
    eventBus: EventBus,
    enabled = true
  ) {
    super(
      id,
      'obs',
      name,
      { ...DEFAULT_OPTIONS, ...options },
      eventBus,
      enabled
    );
    
    this.obs = new OBSWebSocket();
    this.eventEmitter = new EventEmitter();
    
    // Set up event forwarding
    this.setupForwardedEvents();
  }

  protected async connectImplementation(): Promise<void> {
    const options = this.options as ObsAdapterOptions;
    const connectionString = `ws://${options.host}:${options.port}`;
    
    // Update status to connecting
    this.updateStatus(AdapterStatus.CONNECTING, 'Connecting to OBS...');
    
    try {
      // Connect to OBS websocket
      const { obsWebSocketVersion } = await this.obs.connect(
        connectionString,
        options.password
      );
      
      // Reset reconnection state and interval on successful connection
      this.isReconnecting = false;
      options.reconnectInterval = DEFAULT_OPTIONS.reconnectInterval;
      
      // Log the connection
      console.log(`Connected to OBS WebSocket v${obsWebSocketVersion}`);
      
      // Set up event listeners
      this.setupEventListeners();
      
      // Get initial state
      await this.getInitialState();
      
      // Update status to connected explicitly
      this.updateStatus(AdapterStatus.CONNECTED, `Connected to OBS WebSocket v${obsWebSocketVersion}`);
      
    } catch (error) {
      console.error('Failed to connect to OBS:', error);
      
      // Update status to error
      this.updateStatus(AdapterStatus.ERROR, 'Failed to connect to OBS', error as Error);
      
      // Set up reconnection if enabled and not already reconnecting
      if (options.autoReconnect && !this.isReconnecting) {
        this.setupReconnection();
      }
      
      throw error;
    }
  }

  protected async disconnectImplementation(): Promise<void> {
    // Update status to disconnecting
    this.updateStatus(AdapterStatus.DISCONNECTED, 'Disconnecting from OBS...');
    
    // Clear any reconnect timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    
    // Reset reconnection state
    this.isReconnecting = false;
    
    // Remove event listeners
    this.removeEventListeners();
    
    // Disconnect from OBS
    this.obs.disconnect();
    
    // Update status to disconnected explicitly
    this.updateStatus(AdapterStatus.DISCONNECTED, 'Disconnected from OBS');
  }

  protected async disposeImplementation(): Promise<void> {
    // Clean up event emitter
    this.eventEmitter.removeAllListeners();
    
    // Make sure we're disconnected
    if (this.obs) {
      this.obs.disconnect();
    }
  }

  /**
   * Set up event listeners for OBS WebSocket
   */
  private setupEventListeners(): void {
    // Connection events
    this.obs.on('ConnectionOpened', this.handleConnectionOpened.bind(this));
    this.obs.on('ConnectionClosed', this.handleConnectionClosed.bind(this));
    this.obs.on('ConnectionError', this.handleConnectionError.bind(this));
    
    // Scene events
    this.obs.on('CurrentProgramSceneChanged', this.handleSceneChanged.bind(this));
    this.obs.on('SceneListChanged', this.handleSceneListChanged.bind(this));
    this.obs.on('SceneItemEnableStateChanged', this.handleSceneItemChanged.bind(this));
    
    // Streaming events
    this.obs.on('StreamStateChanged', this.handleStreamStateChanged.bind(this));
    
    // Recording events
    this.obs.on('RecordStateChanged', this.handleRecordStateChanged.bind(this));
    
    // Virtual camera events
    this.obs.on('VirtualcamStateChanged', this.handleVirtualCamStateChanged.bind(this));
    
    // Collection events
    this.obs.on('CurrentSceneCollectionChanged', this.handleSceneCollectionChanged.bind(this));
  }

  /**
   * Remove all event listeners
   */
  private removeEventListeners(): void {
    this.obs.removeAllListeners();
  }

  /**
   * Set up forwarded events
   */
  private setupForwardedEvents(): void {
    // Add listeners for adapter-specific events
    Object.keys(OBS_EVENT_TYPE_MAP).forEach(internalEventType => {
      this.eventEmitter.on(internalEventType, (data) => {
        const standardEventType = OBS_EVENT_TYPE_MAP[internalEventType];
        this.publishEvent(standardEventType, data);
      });
    });
  }

  /**
   * Get initial state from OBS
   */
  private async getInitialState(): Promise<void> {
    try {
      // Get current scene
      const { currentProgramSceneName } = await this.obs.call('GetCurrentProgramScene');
      this.currentScene = currentProgramSceneName;
      
      // Get scene list
      const { scenes } = await this.obs.call('GetSceneList');
      this.scenes = Array.isArray(scenes) 
        ? scenes
            .filter(scene => scene && typeof scene === 'object' && 'sceneName' in scene)
            .map(scene => scene.sceneName as string) 
        : [];
      
      // Get streaming status
      const { outputActive: streaming } = await this.obs.call('GetStreamStatus');
      this.isStreaming = streaming;
      
      // Get recording status
      const { outputActive: recording } = await this.obs.call('GetRecordStatus');
      this.isRecording = recording;
      
      // Get virtual camera status
      const { outputActive: virtualCam } = await this.obs.call('GetVirtualCamStatus');
      this.isVirtualCamActive = virtualCam;
      
      // Publish initial state
      this.publishEvent('scene_switched', { sceneName: this.currentScene });
      this.publishEvent('scene_list_changed', { scenes: this.scenes });
      
      if (this.isStreaming) {
        this.publishEvent('streaming_started', { active: true });
      }
      
      if (this.isRecording) {
        this.publishEvent('recording_started', { active: true });
      }
      
      if (this.isVirtualCamActive) {
        this.publishEvent('virtual_cam_started', { active: true });
      }
      
    } catch (error) {
      console.error('Failed to get initial OBS state:', error);
    }
  }

  /**
   * Set up reconnection logic
   */
  private setupReconnection(): void {
    const options = this.options as ObsAdapterOptions;
    
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }
    
    // Set the reconnecting flag
    this.isReconnecting = true;
    
    this.reconnectTimer = setTimeout(async () => {
      try {
        // Only log the first reconnection attempt to avoid spam
        if (options.reconnectInterval === DEFAULT_OPTIONS.reconnectInterval) {
          console.log(`Attempting to reconnect to OBS...`);
        }
        
        await this.connect();
        // Connection succeeded, reset the reconnecting flag
        this.isReconnecting = false;
      } catch (error) {
        // Only log detailed error in development mode to avoid spam
        if (process.env.NODE_ENV === 'development') {
          console.error('Reconnection attempt failed:', error);
        }
        
        // Increase the interval for next retry (exponential backoff)
        options.reconnectInterval = Math.min(options.reconnectInterval * 1.5, 30000);
        
        // Try again with increasing interval (exponential backoff)
        this.setupReconnection();
      }
    }, options.reconnectInterval);
  }

  /**
   * Publish an OBS event to the event bus
   */
  private publishEvent(eventType: ObsEventType, data: any): void {
    this.eventBus.publish(
      createObsEvent(
        eventType,
        { ...data },
        this.id,
        this.name
      )
    );
  }

  // Event handlers
  // Track if we're currently trying to reconnect
  private isReconnecting = false;
  // Track connection state to avoid duplicate logs
  private connectionState = 'disconnected';

  private handleConnectionOpened(): void {
    // Log only if transitioning from a different state
    if (this.connectionState !== 'connected') {
      console.log('OBS WebSocket connection opened');
      this.connectionState = 'connected';
    }
  }

  private handleConnectionClosed(): void {
    // Log only if transitioning from a different state
    if (this.connectionState !== 'disconnected') {
      console.log('OBS WebSocket connection closed');
      this.connectionState = 'disconnected';
      
      // Set up reconnection if enabled and not already reconnecting
      if (this.options.autoReconnect && !this.isReconnecting) {
        this.setupReconnection();
      }
    }
  }

  private handleConnectionError(error: Error | unknown): void {
    // Only log detailed error in development mode to avoid log spam
    if (process.env.NODE_ENV === 'development') {
      console.error('OBS WebSocket connection error:', error);
    } else {
      console.error('OBS WebSocket connection error');
    }
    
    this.updateStatus(AdapterStatus.ERROR, 'OBS connection error', error instanceof Error ? error : new Error(String(error)));
    this.connectionState = 'error';
    
    // Set up reconnection if enabled and not already reconnecting
    if (this.options.autoReconnect && !this.isReconnecting) {
      this.setupReconnection();
    }
  }

  private handleSceneChanged(data: { sceneName: string }): void {
    this.currentScene = data.sceneName;
    this.eventEmitter.emit('scene_switched', { sceneName: data.sceneName });
  }

  private handleSceneListChanged(data: any): void {
    // Safely handle the scene data which might have various structures
    if (data && data.scenes && Array.isArray(data.scenes)) {
      this.scenes = data.scenes
        .filter(scene => scene && typeof scene === 'object' && 'sceneName' in scene)
        .map(scene => scene.sceneName);
    } else {
      this.scenes = [];
    }
    
    this.eventEmitter.emit('scene_list_changed', { scenes: this.scenes });
  }

  private handleSceneItemChanged(data: { 
    sceneName: string;
    sceneItemId: number;
    sceneItemEnabled: boolean;
  }): void {
    this.eventEmitter.emit('source_changed', {
      sceneName: data.sceneName,
      sourceId: data.sceneItemId,
      enabled: data.sceneItemEnabled
    });
  }

  private handleStreamStateChanged(data: { outputActive: boolean }): void {
    this.isStreaming = data.outputActive;
    
    if (data.outputActive) {
      this.eventEmitter.emit('streaming_started', { active: true });
    } else {
      this.eventEmitter.emit('streaming_stopped', { active: false });
    }
  }

  private handleRecordStateChanged(data: { outputActive: boolean }): void {
    this.isRecording = data.outputActive;
    
    if (data.outputActive) {
      this.eventEmitter.emit('recording_started', { active: true });
    } else {
      this.eventEmitter.emit('recording_stopped', { active: false });
    }
  }

  private handleVirtualCamStateChanged(data: { outputActive: boolean }): void {
    this.isVirtualCamActive = data.outputActive;
    
    if (data.outputActive) {
      this.eventEmitter.emit('virtual_cam_started', { active: true });
    } else {
      this.eventEmitter.emit('virtual_cam_stopped', { active: false });
    }
  }

  private handleSceneCollectionChanged(data: { sceneCollectionName: string }): void {
    this.eventEmitter.emit('scene_collection_changed', { 
      collectionName: data.sceneCollectionName 
    });
  }

  /**
   * Public methods for controlling OBS
   */
  
  /**
   * Switch to a specific scene
   */
  async switchScene(sceneName: string): Promise<void> {
    try {
      await this.obs.call('SetCurrentProgramScene', { sceneName });
    } catch (error) {
      console.error(`Failed to switch to scene "${sceneName}":`, error);
      throw error;
    }
  }

  /**
   * Start streaming
   */
  async startStreaming(): Promise<void> {
    try {
      await this.obs.call('StartStream');
    } catch (error) {
      console.error('Failed to start streaming:', error);
      throw error;
    }
  }

  /**
   * Stop streaming
   */
  async stopStreaming(): Promise<void> {
    try {
      await this.obs.call('StopStream');
    } catch (error) {
      console.error('Failed to stop streaming:', error);
      throw error;
    }
  }

  /**
   * Start recording
   */
  async startRecording(): Promise<void> {
    try {
      await this.obs.call('StartRecord');
    } catch (error) {
      console.error('Failed to start recording:', error);
      throw error;
    }
  }

  /**
   * Stop recording
   */
  async stopRecording(): Promise<void> {
    try {
      await this.obs.call('StopRecord');
    } catch (error) {
      console.error('Failed to stop recording:', error);
      throw error;
    }
  }

  /**
   * Start virtual camera
   */
  async startVirtualCamera(): Promise<void> {
    try {
      await this.obs.call('StartVirtualCam');
    } catch (error) {
      console.error('Failed to start virtual camera:', error);
      throw error;
    }
  }

  /**
   * Stop virtual camera
   */
  async stopVirtualCamera(): Promise<void> {
    try {
      await this.obs.call('StopVirtualCam');
    } catch (error) {
      console.error('Failed to stop virtual camera:', error);
      throw error;
    }
  }

  /**
   * Get the current scene name
   */
  getCurrentScene(): string {
    return this.currentScene;
  }

  /**
   * Get the list of available scenes
   */
  getScenes(): string[] {
    return [...this.scenes];
  }

  /**
   * Check if streaming is active
   */
  isStreamingActive(): boolean {
    return this.isStreaming;
  }

  /**
   * Check if recording is active
   */
  isRecordingActive(): boolean {
    return this.isRecording;
  }

  /**
   * Check if virtual camera is active
   */
  isVirtualCameraActive(): boolean {
    return this.isVirtualCamActive;
  }
}