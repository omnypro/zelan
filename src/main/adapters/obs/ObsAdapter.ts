import OBSWebSocket from 'obs-websocket-js'
import { EventEmitter } from 'events'
import { BaseAdapter } from '@s/adapters/base'
import { EventBus } from '@s/core/bus'
import { createObsEvent, ObsEventType } from '@s/core/events'
import { AdapterStatus } from '@s/adapters/interfaces/AdapterStatus'
import { isString, isNumber, isBoolean, createObjectValidator } from '@s/utils/type-guards'
import { AdapterConfig } from '@s/adapters/interfaces/ServiceAdapter'
import { getErrorService } from '@m/services/errors'
import { ApplicationError, ErrorCategory, ErrorSeverity } from '@s/errors'
import { getLoggingService, ComponentLogger } from '@m/services/logging'

/**
 * OBS adapter options
 */
export interface ObsAdapterOptions {
  host: string
  port: number
  password?: string
  reconnectInterval: number
  autoReconnect: boolean
}

/**
 * Default options for the OBS adapter
 */
const DEFAULT_OPTIONS: ObsAdapterOptions = {
  host: 'localhost',
  port: 4455,
  reconnectInterval: 5000,
  autoReconnect: true
}

/**
 * OBS adapter specific event type mapping
 * This maps internal event types to the standard ObsEventType enum
 */
const OBS_EVENT_TYPE_MAP: Record<string, ObsEventType> = {
  scene_switched: ObsEventType.SCENE_SWITCHED,
  streaming_started: ObsEventType.STREAMING_STARTED,
  streaming_stopped: ObsEventType.STREAMING_STOPPED,
  recording_started: ObsEventType.RECORDING_STARTED,
  recording_stopped: ObsEventType.RECORDING_STOPPED,
  source_changed: ObsEventType.SOURCE_CHANGED,
  scene_collection_changed: ObsEventType.SCENE_COLLECTION_CHANGED,
  scene_list_changed: ObsEventType.SCENE_LIST_CHANGED,
  virtual_cam_started: ObsEventType.VIRTUAL_CAM_STARTED,
  virtual_cam_stopped: ObsEventType.VIRTUAL_CAM_STOPPED
}

/**
 * OBS adapter for connecting to OBS Studio via websocket
 */
export class ObsAdapter extends BaseAdapter {
  private obs: OBSWebSocket
  private reconnectTimer?: NodeJS.Timeout
  private eventEmitter: EventEmitter
  private isStreaming: boolean = false
  private isRecording: boolean = false
  private isVirtualCamActive: boolean = false
  private currentScene: string = ''
  private scenes: string[] = []
  private logger: ComponentLogger

  constructor(
    id: string,
    name: string,
    options: Partial<ObsAdapterOptions>,
    eventBus: EventBus,
    enabled = true
  ) {
    super(id, 'obs', name, { ...DEFAULT_OPTIONS, ...options }, eventBus, enabled)

    this.obs = new OBSWebSocket()
    this.eventEmitter = new EventEmitter()
    this.logger = getLoggingService().createLogger(`ObsAdapter:${id}`)

    // Set up event forwarding
    this.setupForwardedEvents()
  }

  /**
   * Type guard for ObsAdapterOptions
   */
  private static isObsAdapterOptions = createObjectValidator<ObsAdapterOptions>({
    host: isString,
    port: isNumber,
    password: (val) => val === undefined || isString(val),
    reconnectInterval: isNumber,
    autoReconnect: isBoolean
  })

  /**
   * Get the options with proper typing
   */
  private getTypedOptions(): ObsAdapterOptions {
    // Use type guard to validate options at runtime
    if (!ObsAdapter.isObsAdapterOptions(this.options)) {
      this.logger.warn('Invalid ObsAdapter options, using defaults', this.options)
      return { ...DEFAULT_OPTIONS }
    }
    return this.options
  }

  protected async connectImplementation(): Promise<void> {
    const options = this.getTypedOptions()
    const connectionString = `ws://${options.host}:${options.port}`

    // Update status to connecting
    this.updateStatus(AdapterStatus.CONNECTING, 'Connecting to OBS...')

    try {
      // Connect to OBS websocket
      const { obsWebSocketVersion } = await this.obs.connect(connectionString, options.password)

      // Reset reconnection state and interval on successful connection
      this.isReconnecting = false
      options.reconnectInterval = DEFAULT_OPTIONS.reconnectInterval

      // Log the connection
      this.logger.info(`Connected to OBS WebSocket v${obsWebSocketVersion}`)

      // Set up event listeners with proper subscriptions
      this.setupEventListeners()

      // Get initial state
      await this.getInitialState()

      // Update status to connected explicitly
      this.updateStatus(
        AdapterStatus.CONNECTED,
        `Connected to OBS WebSocket v${obsWebSocketVersion}`
      )
    } catch (error) {
      // Report error through the error service
      try {
        const errorService = getErrorService()
        errorService.reportError(error instanceof Error ? error : new Error(String(error)), {
          component: 'ObsAdapter',
          operation: 'connect',
          adapterId: this.id,
          adapterName: this.name,
          connectionString,
          recoverable: options.autoReconnect
        })
      } catch (serviceError) {
        // Fallback to console if error service isn't available
        this.logger.error('Failed to connect to OBS', {
          error: error instanceof Error ? error.message : String(error)
        })
      }

      // Update status to error
      this.updateStatus(AdapterStatus.ERROR, 'Failed to connect to OBS', error as Error)

      // Set up reconnection if enabled and not already reconnecting
      if (options.autoReconnect && !this.isReconnecting) {
        this.setupReconnection()
      }

      throw error
    }
  }

  protected async disconnectImplementation(): Promise<void> {
    // Update status to disconnecting
    this.updateStatus(AdapterStatus.DISCONNECTED, 'Disconnecting from OBS...')

    // Clear any reconnect timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = undefined
    }

    // Reset reconnection state
    this.isReconnecting = false
    this.consecutiveReconnectAttempts = 0

    // Remove event listeners
    this.removeEventListeners()

    try {
      // Disconnect from OBS - only if we have an active connection
      if (this.connectionState === 'connected') {
        this.obs.disconnect()
      }
    } catch (error) {
      // Ignore errors during disconnect
    }

    // Update status to disconnected explicitly
    this.updateStatus(AdapterStatus.DISCONNECTED, 'Disconnected from OBS')
  }

  protected async disposeImplementation(): Promise<void> {
    // Clean up event emitter
    this.eventEmitter.removeAllListeners()

    // Clear any reconnect timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = undefined
    }

    // Reset reconnection state
    this.isReconnecting = false
    this.consecutiveReconnectAttempts = 0

    try {
      // Make sure we're disconnected
      if (this.obs) {
        this.obs.disconnect()
      }
    } catch (error) {
      // Ignore errors during disposal
    }
  }

  /**
   * Set up event listeners for OBS WebSocket
   */
  private setupEventListeners(): void {
    // Remove any existing listeners first to prevent duplicates
    this.removeEventListeners()

    // Connection events
    this.obs.on('ConnectionOpened', this.handleConnectionOpened.bind(this))
    this.obs.on('ConnectionClosed', this.handleConnectionClosed.bind(this))
    this.obs.on('ConnectionError', this.handleConnectionError.bind(this))

    // Scene events
    this.obs.on('CurrentProgramSceneChanged', this.handleSceneChanged.bind(this))
    this.obs.on('SceneListChanged', this.handleSceneListChanged.bind(this))
    this.obs.on('SceneItemEnableStateChanged', this.handleSceneItemChanged.bind(this))

    // Streaming events
    this.obs.on('StreamStateChanged', this.handleStreamStateChanged.bind(this))

    // Recording events
    this.obs.on('RecordStateChanged', this.handleRecordStateChanged.bind(this))

    // Virtual camera events
    this.obs.on('VirtualcamStateChanged', this.handleVirtualCamStateChanged.bind(this))

    // Collection events
    this.obs.on('CurrentSceneCollectionChanged', this.handleSceneCollectionChanged.bind(this))
  }

  /**
   * Remove all event listeners
   */
  private removeEventListeners(): void {
    this.obs.removeAllListeners()
  }

  /**
   * Set up forwarded events
   */
  private setupForwardedEvents(): void {
    // Add listeners for adapter-specific events
    Object.keys(OBS_EVENT_TYPE_MAP).forEach((internalEventType) => {
      this.eventEmitter.on(internalEventType, (data) => {
        const standardEventType = OBS_EVENT_TYPE_MAP[internalEventType]
        this.publishEvent(standardEventType, data)
      })
    })
  }

  /**
   * Get initial state from OBS
   */
  private async getInitialState(): Promise<void> {
    try {
      // Get current scene
      const { currentProgramSceneName } = await this.obs.call('GetCurrentProgramScene')
      this.currentScene = currentProgramSceneName

      // Get scene list
      const { scenes } = await this.obs.call('GetSceneList')
      this.scenes = Array.isArray(scenes)
        ? scenes
            .filter((scene) => scene && typeof scene === 'object' && 'sceneName' in scene)
            .map((scene) => scene.sceneName as string)
        : []

      // Get streaming status
      const { outputActive: streaming } = await this.obs.call('GetStreamStatus')
      this.isStreaming = streaming

      // Get recording status
      const { outputActive: recording } = await this.obs.call('GetRecordStatus')
      this.isRecording = recording

      // Get virtual camera status
      const { outputActive: virtualCam } = await this.obs.call('GetVirtualCamStatus')
      this.isVirtualCamActive = virtualCam

      // Publish initial state
      this.publishEvent('scene_switched', { sceneName: this.currentScene })
      this.publishEvent('scene_list_changed', { scenes: this.scenes })

      if (this.isStreaming) {
        this.publishEvent('streaming_started', { active: true })
      }

      if (this.isRecording) {
        this.publishEvent('recording_started', { active: true })
      }

      if (this.isVirtualCamActive) {
        this.publishEvent('virtual_cam_started', { active: true })
      }
    } catch (error) {
      this.logger.error('Failed to get initial OBS state', {
        error: error instanceof Error ? error.message : String(error)
      })
    }
  }

  /**
   * Set up reconnection logic
   */
  /**
   * Maximum number of consecutive reconnection attempts before backing off
   */
  private static MAX_CONSECUTIVE_ATTEMPTS = 5

  /**
   * Count of consecutive reconnection attempts
   */
  private consecutiveReconnectAttempts = 0

  /**
   * Set up reconnection logic with exponential backoff
   */
  private setupReconnection(): void {
    // Clean up any existing reconnection timer
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = undefined
    }

    // If we've exceeded the maximum consecutive attempts, stop aggressive reconnection
    if (this.consecutiveReconnectAttempts >= ObsAdapter.MAX_CONSECUTIVE_ATTEMPTS) {
      this.logger.info(`Maximum reconnection attempts reached. Backing off to longer interval.`)

      // Use a much longer interval for subsequent attempts
      this.reconnectTimer = setTimeout(() => {
        // Reset the counter to allow for fresh attempts if this one fails
        this.consecutiveReconnectAttempts = 0
        this.attemptReconnection()
      }, 30000) // 30 seconds

      return
    }

    // Calculate backoff delay: 1s, 2s, 4s, 8s, 16s
    const delay = Math.min(1000 * Math.pow(2, this.consecutiveReconnectAttempts), 16000)

    // Set the reconnecting flag
    this.isReconnecting = true

    this.reconnectTimer = setTimeout(() => {
      this.attemptReconnection()
    }, delay)
  }

  /**
   * Attempt a single reconnection
   */
  private async attemptReconnection(): Promise<void> {
    try {
      // Increment the attempt counter
      this.consecutiveReconnectAttempts++

      // Only log every few attempts to avoid spam
      if (this.consecutiveReconnectAttempts === 1 || this.consecutiveReconnectAttempts % 5 === 0) {
        this.logger.info(
          `Attempting to reconnect to OBS... (attempt ${this.consecutiveReconnectAttempts})`
        )
      }

      await this.connect()

      // Connection succeeded
      this.isReconnecting = false
      this.consecutiveReconnectAttempts = 0
    } catch (error) {
      // Report error through the error service
      try {
        const errorService = getErrorService()
        errorService.reportError(error instanceof Error ? error : new Error(String(error)), {
          component: 'ObsAdapter',
          operation: 'reconnect',
          adapterId: this.id,
          adapterName: this.name,
          attemptNumber: this.consecutiveReconnectAttempts,
          recoverable: true,
          severity: ErrorSeverity.WARNING
        })
      } catch (serviceError) {
        // Fallback to console if error service isn't available
        if (process.env.NODE_ENV === 'development') {
          this.logger.error('Reconnection attempt failed', {
            error: error instanceof Error ? error.message : String(error)
          })
        }
      }

      // Schedule next reconnection attempt
      this.setupReconnection()
    }
  }

  /**
   * Validate a configuration update specifically for OBS adapter
   * @throws Error if the configuration is invalid
   */
  protected override validateConfigUpdate(config: Partial<AdapterConfig>): void {
    // First validate using the base class implementation
    super.validateConfigUpdate(config)

    // Then perform OBS-specific validation
    if (config.options) {
      // Validate host if provided
      if ('host' in config.options && !isString(config.options.host)) {
        throw new ApplicationError(
          `Invalid OBS host: ${config.options.host}, expected string`,
          ErrorCategory.VALIDATION,
          ErrorSeverity.ERROR,
          {
            component: 'ObsAdapter',
            adapterId: this.id,
            fieldName: 'host',
            receivedValue: config.options.host,
            expectedType: 'string'
          }
        )
      }

      // Validate port if provided
      if ('port' in config.options && !isNumber(config.options.port)) {
        throw new ApplicationError(
          `Invalid OBS port: ${config.options.port}, expected number`,
          ErrorCategory.VALIDATION,
          ErrorSeverity.ERROR,
          {
            component: 'ObsAdapter',
            adapterId: this.id,
            fieldName: 'port',
            receivedValue: config.options.port,
            expectedType: 'number'
          }
        )
      }

      // Validate password if provided (can be undefined or string)
      if (
        'password' in config.options &&
        config.options.password !== undefined &&
        !isString(config.options.password)
      ) {
        throw new ApplicationError(
          `Invalid OBS password, expected string or undefined`,
          ErrorCategory.VALIDATION,
          ErrorSeverity.ERROR,
          {
            component: 'ObsAdapter',
            adapterId: this.id,
            fieldName: 'password',
            expectedType: 'string or undefined'
          }
        )
      }

      // Validate reconnect interval if provided
      if ('reconnectInterval' in config.options && !isNumber(config.options.reconnectInterval)) {
        throw new ApplicationError(
          `Invalid reconnect interval: ${config.options.reconnectInterval}, expected number`,
          ErrorCategory.VALIDATION,
          ErrorSeverity.ERROR,
          {
            component: 'ObsAdapter',
            adapterId: this.id,
            fieldName: 'reconnectInterval',
            receivedValue: config.options.reconnectInterval,
            expectedType: 'number'
          }
        )
      }

      // Validate auto reconnect if provided
      if ('autoReconnect' in config.options && !isBoolean(config.options.autoReconnect)) {
        throw new ApplicationError(
          `Invalid auto reconnect value: ${config.options.autoReconnect}, expected boolean`,
          ErrorCategory.VALIDATION,
          ErrorSeverity.ERROR,
          {
            component: 'ObsAdapter',
            adapterId: this.id,
            fieldName: 'autoReconnect',
            receivedValue: config.options.autoReconnect,
            expectedType: 'boolean'
          }
        )
      }
    }
  }

  /**
   * Publish an OBS event to the event bus
   */
  private publishEvent(eventType: string, data: any): void {
    // Convert raw OBS event type string to our enum type if it exists in the map
    const mappedType = OBS_EVENT_TYPE_MAP[eventType] || (eventType as ObsEventType)

    this.eventBus.publish(createObsEvent(mappedType, { ...data }, this.id, this.name))
  }

  // Event handlers
  // Track if we're currently trying to reconnect
  private isReconnecting = false
  // Track connection state to avoid duplicate logs
  private connectionState = 'disconnected'

  private handleConnectionOpened(): void {
    // Log only if transitioning from a different state
    if (this.connectionState !== 'connected') {
      this.logger.info('OBS WebSocket connection opened')
      this.connectionState = 'connected'
    }
  }

  private handleConnectionClosed(): void {
    // Log only if transitioning from a different state
    if (this.connectionState !== 'disconnected') {
      this.logger.info('OBS WebSocket connection closed')
      this.connectionState = 'disconnected'

      // If we have a pending reconnection, don't schedule another one
      if (this.reconnectTimer) {
        this.logger.info('Reconnection already scheduled, skipping additional reconnect')
        return
      }

      // Set up reconnection if enabled and not already in the process of reconnecting
      const options = this.getTypedOptions()
      if (options.autoReconnect && !this.isReconnecting) {
        this.logger.info('Scheduling reconnection after connection closed')
        this.setupReconnection()
      }
    }
  }

  private handleConnectionError(error: Error | unknown): void {
    // Only log detailed error in development mode to avoid log spam
    if (process.env.NODE_ENV === 'development') {
      this.logger.error('OBS WebSocket connection error', {
        error: error instanceof Error ? error.message : String(error)
      })
    } else {
      this.logger.error('OBS WebSocket connection error')
    }

    this.updateStatus(
      AdapterStatus.ERROR,
      'OBS connection error',
      error instanceof Error ? error : new Error(String(error))
    )
    this.connectionState = 'error'

    // If we already have a reconnection scheduled, don't create another one
    if (this.reconnectTimer) {
      this.logger.info('Reconnection already scheduled after error, skipping additional reconnect')
      return
    }

    // Set up reconnection if enabled and not already reconnecting
    const options = this.getTypedOptions()
    if (options.autoReconnect && !this.isReconnecting) {
      this.logger.info('Scheduling reconnection after error')
      this.setupReconnection()
    }
  }

  private handleSceneChanged(data: { sceneName: string }): void {
    this.currentScene = data.sceneName
    this.eventEmitter.emit('scene_switched', { sceneName: data.sceneName })
  }

  private handleSceneListChanged(data: any): void {
    // Safely handle the scene data which might have various structures
    if (data && data.scenes && Array.isArray(data.scenes)) {
      this.scenes = data.scenes
        .filter((scene) => scene && typeof scene === 'object' && 'sceneName' in scene)
        .map((scene) => scene.sceneName)
    } else {
      this.scenes = []
    }

    this.eventEmitter.emit('scene_list_changed', { scenes: this.scenes })
  }

  private handleSceneItemChanged(data: {
    sceneName: string
    sceneItemId: number
    sceneItemEnabled: boolean
  }): void {
    this.eventEmitter.emit('source_changed', {
      sceneName: data.sceneName,
      sourceId: data.sceneItemId,
      enabled: data.sceneItemEnabled
    })
  }

  private handleStreamStateChanged(data: { outputActive: boolean }): void {
    this.isStreaming = data.outputActive

    if (data.outputActive) {
      this.eventEmitter.emit('streaming_started', { active: true })
    } else {
      this.eventEmitter.emit('streaming_stopped', { active: false })
    }
  }

  private handleRecordStateChanged(data: { outputActive: boolean }): void {
    this.isRecording = data.outputActive

    if (data.outputActive) {
      this.eventEmitter.emit('recording_started', { active: true })
    } else {
      this.eventEmitter.emit('recording_stopped', { active: false })
    }
  }

  private handleVirtualCamStateChanged(data: { outputActive: boolean }): void {
    this.isVirtualCamActive = data.outputActive

    if (data.outputActive) {
      this.eventEmitter.emit('virtual_cam_started', { active: true })
    } else {
      this.eventEmitter.emit('virtual_cam_stopped', { active: false })
    }
  }

  private handleSceneCollectionChanged(data: { sceneCollectionName: string }): void {
    this.eventEmitter.emit('scene_collection_changed', {
      collectionName: data.sceneCollectionName
    })
  }

  /**
   * Public methods for controlling OBS
   */

  /**
   * Switch to a specific scene
   */
  async switchScene(sceneName: string): Promise<void> {
    try {
      await this.obs.call('SetCurrentProgramScene', { sceneName })
    } catch (error) {
      this.logger.error(`Failed to switch to scene "${sceneName}"`, {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Start streaming
   */
  async startStreaming(): Promise<void> {
    try {
      await this.obs.call('StartStream')
    } catch (error) {
      this.logger.error('Failed to start streaming', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Stop streaming
   */
  async stopStreaming(): Promise<void> {
    try {
      await this.obs.call('StopStream')
    } catch (error) {
      this.logger.error('Failed to stop streaming', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Start recording
   */
  async startRecording(): Promise<void> {
    try {
      await this.obs.call('StartRecord')
    } catch (error) {
      this.logger.error('Failed to start recording', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Stop recording
   */
  async stopRecording(): Promise<void> {
    try {
      await this.obs.call('StopRecord')
    } catch (error) {
      this.logger.error('Failed to stop recording', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Start virtual camera
   */
  async startVirtualCamera(): Promise<void> {
    try {
      await this.obs.call('StartVirtualCam')
    } catch (error) {
      this.logger.error('Failed to start virtual camera', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Stop virtual camera
   */
  async stopVirtualCamera(): Promise<void> {
    try {
      await this.obs.call('StopVirtualCam')
    } catch (error) {
      this.logger.error('Failed to stop virtual camera', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  /**
   * Get the current scene name
   */
  getCurrentScene(): string {
    return this.currentScene
  }

  /**
   * Get the list of available scenes
   */
  getScenes(): string[] {
    return [...this.scenes]
  }

  /**
   * Check if streaming is active
   */
  isStreamingActive(): boolean {
    return this.isStreaming
  }

  /**
   * Check if recording is active
   */
  isRecordingActive(): boolean {
    return this.isRecording
  }

  /**
   * Check if virtual camera is active
   */
  isVirtualCameraActive(): boolean {
    return this.isVirtualCamActive
  }
}
