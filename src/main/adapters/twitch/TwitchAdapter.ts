import { BaseAdapter } from '@s/adapters/base'
import { EventBus } from '@s/core/bus'
import { EventCategory } from '@s/types/events'
import { filter } from 'rxjs/operators'
import { createEvent } from '@s/core/events'
import { AdapterStatus } from '@s/adapters/interfaces/AdapterStatus'
import { AdapterConfig } from '@s/adapters/interfaces/ServiceAdapter'
import { isString, isBoolean, createObjectValidator } from '@s/utils/type-guards'
import { ApiClient } from '@twurple/api'
import { AuthProvider as TwurpleAuthProvider, AccessTokenMaybeWithUserId, AccessTokenWithUserId } from '@twurple/auth'
import { EventSubWsListener } from '@twurple/eventsub-ws'
import { getTwitchAuthService } from '@m/services/auth/TwitchAuthService'
import { AuthProvider } from '@s/auth/interfaces'
import { getLoggingService, ComponentLogger } from '@m/services/logging'

/**
 * Twitch adapter options
 *
 * Note about chat messages:
 * The adapter subscribes to chat messages from the broadcaster's perspective,
 * which includes all messages visible to the broadcaster (including moderated content).
 * This is the best approach according to the Twitch EventSub WebSocket API documentation.
 */
export interface TwitchAdapterOptions {
  channelName: string
  includeSubscriptions: boolean
  eventsToTrack: string[]
}

/**
 * Default options for the Twitch adapter
 */
const DEFAULT_OPTIONS: TwitchAdapterOptions = {
  channelName: '',
  includeSubscriptions: false, // Requires additional scopes
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

/**
 * Simple custom auth provider that implements the TwurpleAuthProvider interface
 * This avoids the client_secret requirement of RefreshingAuthProvider by using
 * our own token management.
 */
class CustomTwitchAuthProvider implements TwurpleAuthProvider {
  public readonly clientId: string
  private accessToken: string
  private refreshToken: string
  private scopes: string[]
  private userId: string
  private expiresIn: number
  private obtainmentTimestamp: number
  private refreshCallback?: () => Promise<void>
  private refreshTimer?: NodeJS.Timeout
  private logger: ComponentLogger

  constructor(
    clientId: string,
    accessToken: string,
    refreshToken: string,
    userId: string,
    scopes: string[],
    expiresIn: number
  ) {
    this.clientId = clientId
    this.accessToken = accessToken
    this.refreshToken = refreshToken
    this.userId = userId
    this.scopes = scopes
    this.expiresIn = expiresIn
    this.obtainmentTimestamp = Date.now()
    this.logger = getLoggingService().createLogger('CustomTwitchAuthProvider')

    // Set up refresh timer if we have an expiration
    if (expiresIn > 0) {
      // Schedule refresh for 10 minutes before expiration
      const refreshTime = Math.max(expiresIn - 600, 0) * 1000
      this.refreshTimer = setTimeout(() => this.handleRefresh(), refreshTime)
    }
  }

  /**
   * Set callback for token refresh
   */
  onRefresh(callback: () => Promise<void>): void {
    this.refreshCallback = callback
  }

  /**
   * Handle token refresh internally
   */
  private async handleRefresh(): Promise<void> {
    try {
      if (this.refreshCallback) {
        await this.refreshCallback()
        this.logger.info('Token refreshed through callback')
      }
    } catch (error) {
      this.logger.error('Failed to refresh token', {
        error: error instanceof Error ? error.message : String(error)
      })
    }
  }

  /**
   * Update the underlying token
   */
  updateToken(accessToken: string, refreshToken: string, scopes: string[], expiresIn: number): void {
    // Clear any existing refresh timer
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = undefined
    }

    // Update the token
    this.accessToken = accessToken
    this.refreshToken = refreshToken
    this.scopes = scopes
    this.expiresIn = expiresIn
    this.obtainmentTimestamp = Date.now()

    // Set up a new refresh timer
    if (expiresIn > 0) {
      // Schedule refresh for 10 minutes before expiration
      const refreshTime = Math.max(expiresIn - 600, 0) * 1000
      this.refreshTimer = setTimeout(() => this.handleRefresh(), refreshTime)
    }
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = undefined
    }
  }

  /**
   * Implements required AuthProvider interface methods
   */
  getCurrentScopesForUser(_user: any): string[] {
    return this.scopes
  }

  /**
   * Get access token with user ID (implements AuthProvider interface)
   */
  async getAccessTokenForUser(
    _user: any,
    ..._scopeSets: (string[] | undefined)[]
  ): Promise<AccessTokenWithUserId | null> {
    return {
      accessToken: this.accessToken,
      refreshToken: this.refreshToken || null,
      scope: this.scopes,
      expiresIn: this.expiresIn,
      obtainmentTimestamp: this.obtainmentTimestamp,
      userId: this.userId
    }
  }

  /**
   * Get any access token (implements AuthProvider interface)
   */
  async getAnyAccessToken(_user?: any): Promise<AccessTokenMaybeWithUserId> {
    return {
      accessToken: this.accessToken,
      refreshToken: this.refreshToken || null,
      scope: this.scopes,
      expiresIn: this.expiresIn,
      obtainmentTimestamp: this.obtainmentTimestamp,
      userId: this.userId
    }
  }
}

/**
 * Twitch adapter for connecting to Twitch API
 */
export class TwitchAdapter extends BaseAdapter {
  private apiClient?: ApiClient
  private authProvider?: CustomTwitchAuthProvider
  private eventSubListener?: EventSubWsListener
  private userId?: string
  private channelInfo: Record<string, any> = {}
  private isLive: boolean = false
  private authSubscription?: { unsubscribe: () => void }
  private logger: ComponentLogger

  constructor(id: string, name: string, options: Partial<TwitchAdapterOptions>, eventBus: EventBus, enabled = true) {
    super(id, 'twitch', name, { ...DEFAULT_OPTIONS, ...options }, eventBus, enabled)

    // Initialize component logger
    this.logger = getLoggingService().createLogger(`TwitchAdapter:${id}`)
    this.logger.debug('Twitch adapter initialized')
  }

  // Flag to track if we're in the process of connecting
  private isConnecting: boolean = false

  protected async connectImplementation(): Promise<void> {
    // Prevent duplicate connection attempts
    if (this.isConnecting) {
      this.logger.info('Connection already in progress, skipping duplicate attempt')
      return
    }

    try {
      // Set the connecting flag
      this.isConnecting = true

      const authService = getTwitchAuthService(this.eventBus)
      const authStatus = authService.getStatus(AuthProvider.TWITCH)

      // If not authenticated, just mark as waiting and set up a listener
      if (authStatus.state !== 'authenticated') {
        this.logger.info('Waiting for Twitch authentication')
        this.updateStatus(AdapterStatus.CONNECTING, 'Waiting for Twitch authentication')

        // Clean up any existing subscription
        if (this.authSubscription) {
          this.authSubscription.unsubscribe()
          this.authSubscription = undefined
        }

        // Set up event listener for auth status changes using RxJS observable
        this.authSubscription = this.eventBus
          .getEventsByType('status_changed')
          .pipe(
            // Filter for Twitch auth events
            filter((event) => {
              if (!event.payload) return false
              const payload = event.payload as Record<string, any>
              return (
                event.category === EventCategory.SERVICE &&
                payload.provider === AuthProvider.TWITCH &&
                payload.status === 'authenticated'
              )
            })
          )
          .subscribe(() => {
            this.logger.info('Detected Twitch authentication, reconnecting')
            // Attempt to reconnect when auth becomes available
            this.connect().catch((error) => {
              this.logger.error('Reconnection error', { error: error.message })
            })
          })

        // Reset connecting flag
        this.isConnecting = false
        return // Exit early, don't try to connect yet
      }

      // Get the token from the auth service
      const token = await authService.getToken(AuthProvider.TWITCH)
      if (!token) {
        this.logger.warn('No Twitch authentication token available')
        this.updateStatus(AdapterStatus.ERROR, 'No Twitch authentication token available')
        return // Exit early, don't throw error
      }

      // Create API client using the token
      const { accessToken } = token
      const clientId = process.env.TWITCH_CLIENT_ID || ''

      if (!clientId) {
        this.logger.error('TWITCH_CLIENT_ID environment variable is missing')
        throw new Error('TWITCH_CLIENT_ID environment variable is required')
      }

      // Extract user ID from token metadata
      const userId = token.metadata?.userId
      if (!userId) {
        throw new Error('User ID is required but not available in token metadata')
      }

      // Calculate time until token expiration in seconds
      const expiresIn = Math.floor((token.expiresAt - Date.now()) / 1000)

      // Create our custom auth provider
      // This avoids the client_secret requirement of RefreshingAuthProvider
      this.authProvider = new CustomTwitchAuthProvider(
        clientId,
        accessToken,
        token.refreshToken || '',
        userId,
        token.scope || [],
        expiresIn
      )

      // Set up a listener for token refresh
      this.authProvider.onRefresh(async () => {
        try {
          // Update our token through the auth service
          const result = await authService.refreshToken(AuthProvider.TWITCH)

          if (result.success && result.token) {
            // Update the token in our custom provider
            const newExpiresIn = Math.floor((result.token.expiresAt - Date.now()) / 1000)
            this.authProvider?.updateToken(
              result.token.accessToken,
              result.token.refreshToken || '',
              result.token.scope || [],
              newExpiresIn
            )
            this.logger.info('Token refreshed successfully')
          } else {
            this.logger.error('Token refresh failed', {
              error: result.error?.message || 'Unknown error'
            })
          }
        } catch (error) {
          this.logger.error('Error refreshing token', {
            error: error instanceof Error ? error.message : String(error)
          })
        }
      })

      // Create the API client with our auth provider and disable automatic app access token generation
      this.apiClient = new ApiClient({
        authProvider: this.authProvider,
        // These options help prevent client credential flow errors
        ...({
          disableAutomaticTokenGeneration: true,
          disableAutomaticAuthentication: true,
          disableAutoAuthorization: true
        } as any)
      })

      // Store the user ID for API calls
      this.userId = token.metadata?.userId

      if (!this.userId) {
        // Fallback - get the user ID from the API
        const currentUser = await this.apiClient.users.getUserByName(this.getTypedOptions().channelName)
        if (currentUser) {
          this.userId = currentUser.id
        } else {
          this.isConnecting = false // Reset flag
          throw new Error('Could not determine user ID from token or username')
        }
      }

      // Set up EventSub for real-time updates
      try {
        // First, fetch initial state once
        await this.fetchInitialState()

        // Then set up EventSub for future updates
        await this.setupEventSub()
        this.logger.info('EventSub WebSocket setup successfully')
      } catch (error) {
        this.logger.error('EventSub setup error', {
          error: error instanceof Error ? error.message : String(error)
        })
        this.updateStatus(AdapterStatus.ERROR, 'Failed to set up EventSub')
      } finally {
        // Reset connecting flag when finished (success or error)
        this.isConnecting = false
      }
    } catch (error) {
      // Reset connecting flag on error
      this.isConnecting = false
      this.logger.error('Connection error', {
        error: error instanceof Error ? error.message : String(error)
      })
      throw error
    }
  }

  protected async disconnectImplementation(): Promise<void> {
    // Stop EventSub if it's running
    await this.stopEventSub()

    // Clear cached data
    this.channelInfo = {}
    this.isLive = false
    this.isConnecting = false

    // We don't unsubscribe from auth events here, as we want to
    // be able to reconnect when authentication becomes available

    // Clean up auth provider
    if (this.authProvider) {
      this.authProvider.dispose()
    }

    // Clean up API client
    this.apiClient = undefined
    this.authProvider = undefined
  }

  protected async disposeImplementation(): Promise<void> {
    await this.stopEventSub()

    // Unsubscribe from auth events
    if (this.authSubscription) {
      this.authSubscription.unsubscribe()
      this.authSubscription = undefined
    }

    // Clean up auth provider
    if (this.authProvider) {
      this.authProvider.dispose()
    }

    this.apiClient = undefined
    this.authProvider = undefined
    this.isConnecting = false
  }

  /**
   * Emit an event to the event bus
   */
  private emitEvent(type: string, data: any): void {
    this.eventBus.publish(
      createEvent(
        EventCategory.ADAPTER,
        'data',
        {
          id: this.id,
          type: this.type,
          dataType: type,
          data: {
            ...data,
            channelId: this.userId
          }
        },
        this.id
      )
    )
  }

  /**
   * Fetch initial channel state once at startup
   */
  private async fetchInitialState(): Promise<void> {
    if (!this.apiClient || !this.userId) {
      throw new Error('API client or user ID not available')
    }

    try {
      this.logger.info('Fetching initial channel state')

      // Initialize with basic channel info in case we can't fetch everything
      this.channelInfo = {
        id: this.userId,
        name: '',
        displayName: '',
        title: '',
        gameName: '',
        language: '',
        isLive: false,
        viewers: 0,
        startedAt: null,
        tags: []
      }

      /**
       * Helper to make Twitch API calls safely without causing client credentials errors
       */
      const callTwitchApiSafely = async <T>(apiCall: () => Promise<T>, errorMessage: string): Promise<T | null> => {
        try {
          return await apiCall()
        } catch (err: any) {
          // Check for client credentials errors
          if (
            err?.message?.includes('client_secret') ||
            (err?._body && err?._body.includes('client_secret')) ||
            (err?.status === 400 && err?.message?.includes('Missing params'))
          ) {
            this.logger.warn(`[TwitchAdapter] ${errorMessage} - Client credentials not supported`, {
              error: err.message || 'Unknown error',
              hint: 'Authorization scopes might be insufficient'
            })
          } else {
            // Other API errors
            this.logger.warn(`[TwitchAdapter] ${errorMessage}:`, err)
          }
          return null
        }
      }

      // Get channel information if possible
      const channel = await callTwitchApiSafely(
        () => this.apiClient!.channels.getChannelInfoById(this.userId!),
        'Could not fetch channel info'
      )

      // Update channel info if we got it
      if (channel) {
        this.channelInfo = {
          ...this.channelInfo,
          name: channel.name || '',
          displayName: channel.displayName || '',
          title: channel.title || '',
          gameName: channel.gameName || '',
          language: channel.language || '',
          tags: channel.tags || []
        }
      }

      // Get stream status if possible
      const stream = await callTwitchApiSafely(
        () => this.apiClient!.streams.getStreamByUserId(this.userId!),
        'Could not fetch stream info'
      )

      // Update stream info if we got it
      if (stream) {
        this.isLive = true
        this.channelInfo = {
          ...this.channelInfo,
          isLive: true,
          viewers: stream.viewers || 0,
          startedAt: stream.startDate || null
        }
      }

      // Emit initial state - we use a different format here since this isn't
      // from EventSub but from direct API calls
      this.emitEvent('channel.update', {
        ...this.channelInfo,
        eventType: 'channel.update',
        source: 'initial_state',
        timestamp: Date.now()
      })

      // Emit stream status if online
      if (this.isLive) {
        this.emitEvent('stream.online', {
          ...this.channelInfo,
          eventType: 'stream.online',
          source: 'initial_state',
          timestamp: Date.now()
        })
      }

      this.logger.info('Initial state fetched successfully')
    } catch (error) {
      this.logger.error('Failed to fetch initial state', {
        error: error instanceof Error ? error.message : String(error)
      })
      // Continue anyway, EventSub will update the state as events occur
    }
  }

  /**
   * Set up EventSub WebSocket listener
   */
  private async setupEventSub(): Promise<void> {
    if (!this.apiClient || !this.userId) {
      throw new Error('API client or user ID not available')
    }

    // Create the EventSub listener using our API client
    this.eventSubListener = new EventSubWsListener({
      apiClient: this.apiClient,
      // Add a logger with a custom name
      logger: {
        name: 'twitch-eventsub'
      }
    })

    this.logger.info('Starting EventSub WebSocket listener')
    await this.eventSubListener.start()

    // Subscribe to events
    this.subscribeToEvents()
  }

  /**
   * Subscribe to Twitch events via EventSub
   */
  private subscribeToEvents(): void {
    if (!this.eventSubListener || !this.userId) {
      this.logger.error('Cannot subscribe to events: EventSub not initialized')
      return
    }

    this.logger.info('Subscribing to Twitch events for user', { userId: this.userId })

    // Helper to safely subscribe to stream & channel events (with 1 parameter)
    const safeSubscribe = <T>(
      subscriptionFn: (userId: string, callback: (event: T) => void) => unknown,
      handler: (event: T) => void,
      eventName: string
    ) => {
      try {
        if (typeof subscriptionFn !== 'function') {
          this.logger.error(`Subscription function for ${eventName} is not available`)
          return
        }

        subscriptionFn(this.userId!, handler)
        this.logger.info(`Successfully subscribed to ${eventName} events`)
      } catch (error) {
        this.logger.error(`Failed to subscribe to ${eventName} events`, {
          error: error instanceof Error ? error.message : String(error),
          eventName
        })
        // The subscription failed but we can continue with others
      }
    }

    // Subscribe to stream online events
    safeSubscribe(
      this.eventSubListener.onStreamOnline.bind(this.eventSubListener),
      (event) => {
        this.logger.debug('Stream online event received', { event })
        this.isLive = true
        this.emitEvent('stream.online', {
          ...event, // Pass all original fields
          eventType: 'stream.online',
          timestamp: Date.now()
        })
      },
      'stream.online'
    )

    // Subscribe to stream offline events
    safeSubscribe(
      this.eventSubListener.onStreamOffline.bind(this.eventSubListener),
      (event) => {
        this.logger.debug('Stream offline event received', { event })
        this.isLive = false
        this.emitEvent('stream.offline', {
          ...event, // Pass all original fields
          eventType: 'stream.offline',
          timestamp: Date.now()
        })
      },
      'stream.offline'
    )

    // Subscribe to channel update events
    safeSubscribe(
      this.eventSubListener.onChannelUpdate.bind(this.eventSubListener),
      (event) => {
        this.logger.debug('Channel update event received', { event })

        // Update stored channel info with latest values
        this.channelInfo = {
          ...this.channelInfo,
          title: event.streamTitle,
          gameName: event.categoryName
        }

        // Emit channel update event
        this.emitEvent('channel.update', {
          ...event, // Pass all original fields
          eventType: 'channel.update',
          timestamp: Date.now()
        })
      },
      'channel.update'
    )

    // Subscribe to chat messages
    if (this.eventSubListener.onChannelChatMessage) {
      try {
        // Based on the Twitch EventSub API docs:
        // - broadcaster_user_id: The channel to get chat messages from (required)
        // - user_id: The user perspective to read chat as (required)
        //
        // To get all chat messages visible to the broadcaster (including moderator messages),
        // we use the broadcaster ID for both parameters.
        this.eventSubListener.onChannelChatMessage(
          this.userId, // broadcaster_user_id - the channel
          this.userId, // user_id - view chat as the broadcaster (sees all messages)
          (event) => {
            this.logger.debug('Chat message received')

            // Pass through the original event data with minimal processing
            this.emitEvent('chat.message', {
              ...event,
              eventType: 'chat.message',
              timestamp: Date.now()
            })
          }
        )

        this.logger.info('Successfully subscribed to chat message events')
      } catch (error) {
        this.logger.error('Failed to subscribe to chat message events', {
          error: error instanceof Error ? error.message : String(error)
        })
      }
    } else {
      this.logger.warn('Chat message subscription not available. This might require additional scopes.')
    }

    // Subscribe to cheer events (bits)
    if (this.eventSubListener.onChannelCheer) {
      try {
        this.eventSubListener.onChannelCheer(this.userId, (event) => {
          this.logger.debug('Cheer received with bits', { bits: event.bits })

          // Pass through the original event data with minimal processing
          this.emitEvent('chat.cheer', {
            ...event, // Pass all original fields
            eventType: 'chat.cheer',
            timestamp: Date.now()
          })
        })
        this.logger.info('Successfully subscribed to chat.cheer events')
      } catch (error) {
        this.logger.error('Failed to subscribe to chat.cheer events', {
          error: error instanceof Error ? error.message : String(error)
        })
      }
    } else {
      this.logger.warn('Cheer subscription not available. This might require additional scopes.')
    }

    // Subscribe to subscription events
    if (this.eventSubListener.onChannelSubscription) {
      try {
        this.eventSubListener.onChannelSubscription(this.userId, (event) => {
          this.logger.debug('Subscription event received')

          // Pass through the original event data with minimal processing
          this.emitEvent('chat.subscription', {
            ...event, // Pass all original fields
            eventType: 'chat.subscription',
            timestamp: Date.now()
          })
        })
        this.logger.info('Successfully subscribed to chat.subscription events')
      } catch (error) {
        this.logger.error('Failed to subscribe to chat.subscription events', {
          error: error instanceof Error ? error.message : String(error)
        })
      }
    } else {
      this.logger.warn('Subscription event handler not available. This might require additional scopes.')
    }

    // Subscribe to raid events
    if (this.eventSubListener.onChannelRaidTo) {
      try {
        this.eventSubListener.onChannelRaidTo(this.userId, (event) => {
          this.logger.debug('Raid received with viewers', { viewers: event.viewers })

          // Pass through the original event data with minimal processing
          this.emitEvent('chat.raid', {
            ...event, // Pass all original fields
            eventType: 'chat.raid',
            timestamp: Date.now()
          })
        })
        this.logger.info('Successfully subscribed to chat.raid events')
      } catch (error) {
        this.logger.error('Failed to subscribe to chat.raid events', {
          error: error instanceof Error ? error.message : String(error)
        })
      }
    } else {
      this.logger.warn('Raid event handler not available. This might require additional scopes.')
    }
  }

  /**
   * Stop EventSub WebSocket listener
   */
  private async stopEventSub(): Promise<void> {
    if (this.eventSubListener) {
      this.logger.info('Stopping EventSub WebSocket listener')
      await this.eventSubListener.stop()
      this.eventSubListener = undefined
    }
  }

  /**
   * Validate configuration update for Twitch adapter
   */
  protected override validateConfigUpdate(config: Partial<AdapterConfig>): void {
    // First validate using the base class implementation
    super.validateConfigUpdate(config)

    // Then perform Twitch-specific validation
    if (config.options) {
      // Validate channel name if provided
      if ('channelName' in config.options && !isString(config.options.channelName)) {
        throw new Error(`Invalid channel name: ${config.options.channelName}, expected string`)
      }

      // Validate includeSubscriptions if provided
      if ('includeSubscriptions' in config.options && !isBoolean(config.options.includeSubscriptions)) {
        throw new Error(`Invalid includeSubscriptions value: ${config.options.includeSubscriptions}, expected boolean`)
      }
    }
  }

  /**
   * Type guard for TwitchAdapterOptions
   */
  private static isTwitchAdapterOptions = createObjectValidator<TwitchAdapterOptions>({
    channelName: isString,
    includeSubscriptions: isBoolean,
    eventsToTrack: (value): value is string[] => Array.isArray(value) && value.every((item) => typeof item === 'string')
  })

  /**
   * Get the options with proper typing
   */
  private getTypedOptions(): TwitchAdapterOptions {
    // Use type guard to validate options at runtime
    if (!TwitchAdapter.isTwitchAdapterOptions(this.options)) {
      this.logger.warn('Invalid TwitchAdapter options, using defaults', this.options)
      return { ...DEFAULT_OPTIONS }
    }
    return this.options
  }
}
