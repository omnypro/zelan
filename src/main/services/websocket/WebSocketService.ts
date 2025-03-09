import { WebSocket, WebSocketServer as WSServer } from 'ws'
import { EventBus } from '@s/core/bus'
import { getLoggingService, ComponentLogger } from '@m/services/logging'
import { BaseEvent, EventCategory, SystemEventType } from '@s/types/events'
import { SubscriptionManager } from '@s/utils/subscription-manager'
import { ConfigStore } from '@s/core/config'

/**
 * WebSocket server configuration
 */
export interface WebSocketServiceConfig {
  port: number
  pingInterval: number // in milliseconds
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: WebSocketServiceConfig = {
  port: 8081,
  pingInterval: 30000 // 30 seconds
}

/**
 * WebSocket service for exposing events to external clients
 */
export class WebSocketService {
  private server: WSServer | null = null
  private clients = new Map<WebSocket, { lastActivity: number }>()
  private subscriptionManager = new SubscriptionManager()
  private pingInterval: NodeJS.Timeout | null = null
  private config: WebSocketServiceConfig
  private isRunning = false
  private logger: ComponentLogger
  
  // Singleton instance
  private static instance: WebSocketService | null = null
  
  /**
   * Get the WebSocketService singleton instance
   */
  static getInstance(eventBus: EventBus, configStore?: ConfigStore): WebSocketService {
    if (!WebSocketService.instance) {
      // Get config from store if available
      let config: Partial<WebSocketServiceConfig> = {}
      
      if (configStore) {
        const storedPort = configStore.get('websocket.port', DEFAULT_CONFIG.port) as number
        const storedPingInterval = configStore.get('websocket.pingInterval', DEFAULT_CONFIG.pingInterval) as number
        
        config = {
          port: storedPort,
          pingInterval: storedPingInterval
        }
      }
      
      WebSocketService.instance = new WebSocketService(eventBus, config)
    }
    
    return WebSocketService.instance
  }

  private constructor(
    private eventBus: EventBus,
    config?: Partial<WebSocketServiceConfig>
  ) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.logger = getLoggingService().createLogger('WebSocketService')
  }

  /**
   * Start the WebSocket server
   * @returns true if started successfully, false if already running
   */
  start(): boolean {
    if (this.isRunning) {
      this.logger.info('WebSocket server is already running')
      return false
    }

    try {
      // Create WebSocket server
      this.server = new WSServer({ port: this.config.port })

      // Set up connection handling
      this.server.on('connection', this.handleConnection.bind(this))
      this.server.on('error', this.handleServerError.bind(this))

      // Subscribe to all events
      this.subscriptionManager.add(this.eventBus.events$.subscribe(this.broadcastEvent.bind(this)))

      // Set up ping interval
      this.setupPingInterval()

      this.isRunning = true
      this.logger.info(`WebSocket server started on port ${this.config.port}`)
      
      // Publish event about server starting
      this.eventBus.publish({
        id: `websocket-start-${Date.now()}`,
        timestamp: Date.now(),
        source: {
          id: 'websocket-service',
          name: 'WebSocket Service',
          type: 'system'
        },
        category: EventCategory.SYSTEM,
        type: SystemEventType.INFO,
        data: {
          message: `WebSocket server started on port ${this.config.port}`
        },
        metadata: {
          version: '1.0'
        }
      })
      
      return true
    } catch (error) {
      this.logger.error('Failed to start WebSocket server', {
        error: error instanceof Error ? error.message : String(error),
        port: this.config.port
      })
      this.cleanup()
      return false
    }
  }

  /**
   * Stop the WebSocket server
   */
  stop(): void {
    this.cleanup()
    
    // Publish event about server stopping
    this.eventBus.publish({
      id: `websocket-stop-${Date.now()}`,
      timestamp: Date.now(),
      source: {
        id: 'websocket-service',
        name: 'WebSocket Service',
        type: 'system'
      },
      category: EventCategory.SYSTEM,
      type: SystemEventType.INFO,
      data: {
        message: 'WebSocket server stopped'
      },
      metadata: {
        version: '1.0'
      }
    })
  }
  
  /**
   * Clean up resources
   */
  private cleanup(): void {
    // Clear ping interval
    if (this.pingInterval) {
      clearInterval(this.pingInterval)
      this.pingInterval = null
    }

    // Unsubscribe from all events
    this.subscriptionManager.unsubscribeAll()

    // Close all client connections
    for (const client of this.clients.keys()) {
      try {
        client.terminate()
      } catch (e) {
        // Ignore errors when closing clients
      }
    }
    this.clients.clear()

    // Close server
    if (this.server) {
      try {
        this.server.close()
      } catch (e) {
        // Ignore errors when closing server
      }
      this.server = null
    }

    this.isRunning = false
    this.logger.info('WebSocket server stopped')
  }

  /**
   * Get the server status
   */
  getStatus(): { running: boolean; clientCount: number; port: number } {
    return {
      running: this.isRunning,
      clientCount: this.clients.size,
      port: this.config.port
    }
  }

  /**
   * Set up ping interval to keep connections alive
   */
  private setupPingInterval(): void {
    this.pingInterval = setInterval(() => {
      this.checkConnections()
    }, this.config.pingInterval)
  }

  /**
   * Check client connections and remove stale ones
   */
  private checkConnections(): void {
    const now = Date.now()
    const staleTimeout = this.config.pingInterval * 2.5 // If no activity in 2.5x ping interval, consider stale
    
    for (const [client, data] of this.clients.entries()) {
      if (now - data.lastActivity > staleTimeout) {
        // Client hasn't responded in too long
        this.removeClient(client)
      } else if (client.readyState === WebSocket.OPEN) {
        // Send ping to active clients
        try {
          client.ping()
        } catch (e) {
          this.removeClient(client)
        }
      } else if (client.readyState !== WebSocket.CONNECTING) {
        // Clean up non-connecting clients
        this.removeClient(client)
      }
    }
  }

  /**
   * Handle new WebSocket connection
   */
  private handleConnection(client: WebSocket): void {
    // Add to client collection with timestamp
    this.clients.set(client, { lastActivity: Date.now() })

    // Setup client event listeners
    client.on('close', () => this.removeClient(client))
    client.on('error', () => this.removeClient(client))
    client.on('pong', () => {
      // Update last activity time when client responds
      const clientData = this.clients.get(client)
      if (clientData) {
        clientData.lastActivity = Date.now()
      }
    })
    
    // Also update activity on message
    client.on('message', () => {
      const clientData = this.clients.get(client)
      if (clientData) {
        clientData.lastActivity = Date.now()
      }
    })

    // Send welcome message
    this.sendToClient(client, {
      id: 'welcome',
      timestamp: Date.now(),
      source: {
        id: 'websocket-server',
        name: 'WebSocket Server',
        type: 'system'
      },
      category: EventCategory.SYSTEM,
      type: SystemEventType.INFO,
      data: {
        message: 'Connected to Zelan WebSocket Server',
        clientCount: this.clients.size
      },
      metadata: {
        version: '1.0'
      }
    })

    this.logger.info(`WebSocket client connected`, { clientCount: this.clients.size })
  }

  /**
   * Remove a client 
   */
  private removeClient(client: WebSocket): void {
    if (this.clients.has(client)) {
      try {
        client.terminate()
      } catch (e) {
        // Ignore errors when terminating
      }
      this.clients.delete(client)
      this.logger.info(`WebSocket client disconnected`, { clientCount: this.clients.size })
    }
  }

  /**
   * Handle server error
   */
  private handleServerError(error: Error): void {
    this.logger.error('WebSocket server error', {
      error: error.message,
      stack: error.stack
    })
  }

  /**
   * Broadcast an event to all connected clients
   */
  private broadcastEvent<T>(event: BaseEvent<T>): void {
    if (!this.isRunning || this.clients.size === 0) {
      return
    }

    for (const client of this.clients.keys()) {
      try {
        if (client.readyState === WebSocket.OPEN) {
          this.sendToClient(client, event)
        } else if (client.readyState === WebSocket.CLOSED || client.readyState === WebSocket.CLOSING) {
          this.removeClient(client)
        }
      } catch (e) {
        this.removeClient(client)
      }
    }
  }

  /**
   * Send event to a specific client
   */
  private sendToClient<T>(client: WebSocket, event: BaseEvent<T>): void {
    try {
      const message = JSON.stringify(event, (_, value) => {
        // Handle special cases like functions, circular refs, etc.
        if (typeof value === 'function') {
          return '[Function]'
        }
        return value
      })
      
      client.send(message)
      
      // Update last activity time
      const clientData = this.clients.get(client)
      if (clientData) {
        clientData.lastActivity = Date.now()
      }
    } catch (error) {
      this.logger.error('Error sending message to client', {
        error: error instanceof Error ? error.message : String(error)
      })
      this.removeClient(client)
    }
  }
}