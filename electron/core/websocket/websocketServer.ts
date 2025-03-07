import { WebSocket, WebSocketServer as WSServer } from 'ws'
import { EventBus, BaseEvent } from '../events'
import { BehaviorSubject, Observable, Subject, Subscription, takeUntil, timer } from 'rxjs'
import { z } from 'zod'
import { ConfigManager } from '../config/configManager'
import { WebSocketConfig, WebSocketConfigSchema } from './index'

export const WebSocketMessageSchema = z.object({
  type: z.string(),
  payload: z.any(),
  timestamp: z.number()
})

export type WebSocketMessage = z.infer<typeof WebSocketMessageSchema>

/**
 * WebSocketServer provides a real-time events API for external clients
 * Main process only
 */
export class WebSocketServer {
  private static instance: WebSocketServer
  private server: WSServer | null = null
  private clients: Set<WebSocket> = new Set()
  private eventBus: EventBus = EventBus.getInstance()
  private eventSubscription: Subscription | null = null
  private pingInterval: Subscription | null = null
  private destroy$ = new Subject<void>()
  private config: WebSocketConfig
  private stateSubject = new BehaviorSubject<boolean>(false)

  private constructor(config: Partial<WebSocketConfig> = {}) {
    // Try to load WebSocket config from app settings if available
    try {
      const configManager = ConfigManager.getInstance();
      const storedConfig = configManager.getWebSocketConfig();
      
      // Merge stored config with provided config, with provided config taking precedence
      this.config = WebSocketConfigSchema.parse({
        ...storedConfig,
        ...config
      });
    } catch (error) {
      // Fall back to just using the provided config if ConfigManager is not available
      this.config = WebSocketConfigSchema.parse(config);
    }
  }

  /**
   * Get singleton instance of WebSocketServer
   */
  public static getInstance(config?: Partial<WebSocketConfig>): WebSocketServer {
    if (!WebSocketServer.instance) {
      WebSocketServer.instance = new WebSocketServer(config)
    } else if (config) {
      WebSocketServer.instance.updateConfig(config)
    }

    return WebSocketServer.instance
  }

  /**
   * Start the WebSocket server
   */
  public start(): void {
    if (this.server) {
      console.log('WebSocket server already running')
      return
    }

    try {
      // Create server
      this.server = new WSServer({
        port: this.config.port,
        path: this.config.path
      })

      // Set up event handlers
      this.server.on('connection', this.handleConnection.bind(this))
      this.server.on('error', this.handleServerError.bind(this))

      // Subscribe to events
      this.subscribeToEvents()

      // Start ping interval
      this.startPingInterval()

      // Update state
      this.stateSubject.next(true)

      console.log(`WebSocket server started on port ${this.config.port}`)
    } catch (error) {
      console.error('Failed to start WebSocket server:', error)
      this.stateSubject.next(false)
    }
  }

  /**
   * Stop the WebSocket server
   */
  public stop(): void {
    if (!this.server) {
      return
    }

    // Close all connections
    for (const client of this.clients) {
      client.terminate()
    }
    this.clients.clear()

    // Unsubscribe from events
    this.unsubscribeFromEvents()

    // Stop ping interval
    this.stopPingInterval()

    // Close server
    this.server.close()
    this.server = null

    // Update state
    this.stateSubject.next(false)

    console.log('WebSocket server stopped')
  }

  /**
   * Update server configuration
   */
  public updateConfig(config: Partial<WebSocketConfig>): void {
    const wasRunning = this.isRunning()

    // Stop server if running
    if (wasRunning) {
      this.stop()
    }

    // Update config
    this.config = WebSocketConfigSchema.parse({
      ...this.config,
      ...config
    })

    // Restart server if it was running
    if (wasRunning) {
      this.start()
    }
  }

  /**
   * Check if server is running
   */
  public isRunning(): boolean {
    return this.server !== null
  }

  /**
   * Get server state as an observable
   */
  public state$(): Observable<boolean> {
    return this.stateSubject.asObservable()
  }

  /**
   * Get the number of connected clients
   */
  public getClientCount(): number {
    return this.clients.size
  }

  /**
   * Broadcast a message to all connected clients
   */
  public broadcast(message: WebSocketMessage): void {
    if (!this.isRunning() || this.clients.size === 0) {
      return
    }

    const messageString = JSON.stringify(message)

    for (const client of this.clients) {
      if (client.readyState === WebSocket.OPEN) {
        client.send(messageString)
      }
    }
  }

  /**
   * Handle a new WebSocket connection
   */
  private handleConnection(socket: WebSocket): void {
    // Add to clients set
    this.clients.add(socket)
    console.log(`Client connected. Total clients: ${this.clients.size}`)

    // Set up event handlers
    socket.on('message', (data: string) => this.handleMessage(socket, data))
    socket.on('close', () => this.handleClose(socket))
    socket.on('error', (error: Error) => this.handleClientError(socket, error))

    // Send welcome message
    socket.send(
      JSON.stringify({
        type: 'connection.established',
        payload: { message: 'Connected to Zelan WebSocket Server' },
        timestamp: Date.now()
      })
    )
  }

  /**
   * Handle a WebSocket message
   */
  private handleMessage(socket: WebSocket, data: string): void {
    try {
      const message = JSON.parse(data.toString())

      // Handle ping message
      if (message.type === 'ping') {
        socket.send(
          JSON.stringify({
            type: 'pong',
            payload: {},
            timestamp: Date.now()
          })
        )
      }
    } catch (error) {
      console.error('Error parsing WebSocket message:', error)
    }
  }

  /**
   * Handle a WebSocket connection close
   */
  private handleClose(socket: WebSocket): void {
    // Remove from clients set
    this.clients.delete(socket)
    console.log(`Client disconnected. Total clients: ${this.clients.size}`)
  }

  /**
   * Handle a WebSocket client error
   */
  private handleClientError(socket: WebSocket, error: Error): void {
    console.error('WebSocket client error:', error)

    // Remove from clients set
    this.clients.delete(socket)
  }

  /**
   * Handle a WebSocket server error
   */
  private handleServerError(error: Error): void {
    console.error('WebSocket server error:', error)

    // Stop server
    this.stop()
  }

  /**
   * Subscribe to EventBus events
   */
  private subscribeToEvents(): void {
    // Unsubscribe if already subscribed
    this.unsubscribeFromEvents()

    // Subscribe to all events
    this.eventSubscription = this.eventBus
      .events()
      .pipe(takeUntil(this.destroy$))
      .subscribe((event) => {
        this.broadcastEvent(event)
      })
  }

  /**
   * Unsubscribe from EventBus events
   */
  private unsubscribeFromEvents(): void {
    if (this.eventSubscription) {
      this.eventSubscription.unsubscribe()
      this.eventSubscription = null
    }
  }

  /**
   * Start ping interval to keep connections alive
   */
  private startPingInterval(): void {
    // Stop if already running
    this.stopPingInterval()

    // Start new interval
    this.pingInterval = timer(this.config.pingInterval, this.config.pingInterval)
      .pipe(takeUntil(this.destroy$))
      .subscribe(() => {
        this.pingClients()
      })
  }

  /**
   * Stop ping interval
   */
  private stopPingInterval(): void {
    if (this.pingInterval) {
      this.pingInterval.unsubscribe()
      this.pingInterval = null
    }
  }

  /**
   * Ping all connected clients
   */
  private pingClients(): void {
    const pingMessage = {
      type: 'server.ping',
      payload: {},
      timestamp: Date.now()
    }

    this.broadcast(pingMessage)
  }

  /**
   * Broadcast an event to all connected clients
   */
  private broadcastEvent(event: BaseEvent): void {
    const message: WebSocketMessage = {
      type: event.type,
      payload: event,
      timestamp: Date.now()
    }

    this.broadcast(message)
  }

  /**
   * Clean up resources
   */
  public destroy(): void {
    // Stop server
    this.stop()

    // Complete destroy subject
    this.destroy$.next()
    this.destroy$.complete()
  }
}