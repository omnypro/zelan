import WebSocket, { WebSocketServer } from 'ws'
import { BaseEvent, EventCategory, SystemEventType } from '@s/types/events'
import { createSystemEvent } from '@s/core/events'
import { EventBus } from '@s/core/bus'
import { ConfigStore } from '@s/core/config/ConfigStore'
import { SubscriptionManager } from '@s/utils/subscription-manager'
import { getLoggingService, ComponentLogger } from '@m/services/logging'

interface WebSocketClient extends WebSocket {
  isAlive: boolean
  id: string
  filters?: {
    categories?: string[]
    types?: string[]
    sources?: string[]
  }
}

/**
 * Server status information
 */
export interface WebSocketServerStatus {
  running: boolean
  port: number
  clientCount: number
  uptime: number
}

/**
 * WebSocket server for broadcasting events to clients
 */
export class WebSocketService {
  private server?: WebSocketServer
  private clients: WebSocketClient[] = []
  private pingInterval?: NodeJS.Timeout
  private isRunning = false
  private port: number
  private startTime: number = 0
  private subscriptionManager = new SubscriptionManager()
  private logger: ComponentLogger

  private static instance: WebSocketService

  /**
   * Get the singleton instance
   */
  static getInstance(eventBus: EventBus, configStore: ConfigStore): WebSocketService {
    if (!WebSocketService.instance) {
      WebSocketService.instance = new WebSocketService(eventBus, configStore)
    }
    return WebSocketService.instance
  }

  /**
   * Create a new WebSocket service
   */
  private constructor(
    private eventBus: EventBus,
    configStore: ConfigStore
  ) {
    this.logger = getLoggingService().createLogger('WebSocketService')
    this.port = configStore.getSettings().webSocketPort

    // Listen for settings changes
    this.subscriptionManager.add(
      configStore.settings$().subscribe((settings) => {
        const newPort = settings.webSocketPort
        if (this.port !== newPort) {
          this.port = newPort

          // Restart server if running
          if (this.isRunning) {
            this.stop()
            this.start()
          }
        }
      })
    )

    // Handle app exit
    process.on('exit', () => {
      this.stop()
    })
  }

  /**
   * Start the WebSocket server
   */
  start(): boolean {
    if (this.isRunning) {
      return false
    }

    try {
      this.server = new WebSocketServer({ port: this.port })
      this.startTime = Date.now()

      this.server.on('connection', (socket) => {
        this.handleConnection(socket as WebSocketClient)
      })

      this.server.on('error', (error) => {
        this.logger.error('WebSocket server error', {
          error: error instanceof Error ? error.message : String(error)
        })
        this.eventBus.publish(
          createSystemEvent(SystemEventType.ERROR, 'WebSocket server error', 'error', {
            error: error.message
          })
        )
      })

      // Set up ping interval
      this.pingInterval = setInterval(() => {
        this.pingClients()
      }, 30000)

      this.isRunning = true

      // Subscribe to events
      this.subscriptionManager.add(
        this.eventBus.events$.subscribe((event) => {
          this.broadcastEvent(event)
        })
      )

      // Publish started event
      this.eventBus.publish(
        createSystemEvent(SystemEventType.INFO, `WebSocket server started on port ${this.port}`, 'info', {
          port: this.port
        })
      )

      return true
    } catch (error) {
      this.logger.error('Failed to start WebSocket server', {
        error: error instanceof Error ? error.message : String(error)
      })
      this.eventBus.publish(
        createSystemEvent(SystemEventType.ERROR, 'Failed to start WebSocket server', 'error', {
          error: error instanceof Error ? error.message : String(error)
        })
      )
      return false
    }
  }

  /**
   * Stop the WebSocket server
   */
  stop(): void {
    // Unsubscribe from all event subscriptions
    this.subscriptionManager.unsubscribeAll()

    if (this.pingInterval) {
      clearInterval(this.pingInterval)
      this.pingInterval = undefined
    }

    if (this.server) {
      // Close all client connections
      for (const client of this.clients) {
        client.terminate()
      }

      this.server.close()
      this.server = undefined
    }

    this.clients = []
    this.isRunning = false

    // Publish stopped event
    this.eventBus.publish(createSystemEvent(SystemEventType.INFO, 'WebSocket server stopped', 'info'))
  }

  /**
   * Get current server status
   */
  getStatus(): WebSocketServerStatus {
    return {
      running: this.isRunning,
      port: this.port,
      clientCount: this.clients.length,
      uptime: this.isRunning ? Date.now() - this.startTime : 0
    }
  }

  /**
   * Handle a new client connection
   */
  private handleConnection(socket: WebSocketClient): void {
    // Initialize client properties
    socket.isAlive = true
    socket.id = Date.now().toString(36) + Math.random().toString(36).substr(2, 5)

    // Add to clients collection
    this.clients.push(socket)

    // Send welcome message with server info
    socket.send(
      JSON.stringify({
        type: 'connection',
        id: generateEventId(),
        timestamp: Date.now(),
        category: EventCategory.SYSTEM,
        source: {
          id: 'websocket-server',
          name: 'WebSocket Server',
          type: 'system'
        },
        data: {
          message: 'Connected to Zelan WebSocket server',
          status: 'connected',
          clientId: socket.id,
          serverStatus: this.getStatus()
        }
      })
    )

    // Set up event handlers
    socket.on('close', () => {
      this.handleDisconnection(socket)
    })

    socket.on('error', (error) => {
      this.logger.error('WebSocket client error', {
        error: error instanceof Error ? error.message : String(error)
      })
    })

    socket.on('pong', () => {
      socket.isAlive = true
    })

    // Handle messages from client
    socket.on('message', (data) => {
      this.handleClientMessage(socket, data)
    })

    // Log the connection
    this.logger.info(`WebSocket client connected: ${socket.id}`)

    // Publish event about new connection
    this.eventBus.publish(
      createSystemEvent(SystemEventType.INFO, 'WebSocket client connected', 'info', {
        clientId: socket.id
      })
    )
  }

  /**
   * Handle a client disconnection
   */
  private handleDisconnection(socket: WebSocketClient): void {
    // Remove from clients collection
    this.clients = this.clients.filter((client) => client !== socket)

    // Log the disconnection
    this.logger.info(`WebSocket client disconnected: ${socket.id}`)

    // Publish event about disconnection
    this.eventBus.publish(
      createSystemEvent(SystemEventType.INFO, 'WebSocket client disconnected', 'info', {
        clientId: socket.id
      })
    )
  }

  /**
   * Handle a message from a client
   */
  private handleClientMessage(socket: WebSocketClient, data: WebSocket.Data): void {
    try {
      // Parse the message
      const message = JSON.parse(data.toString())

      // Handle different message types
      switch (message.type) {
        case 'filter':
          this.handleFilterMessage(socket, message)
          break

        case 'ping':
          this.handlePingMessage(socket)
          break

        case 'request':
          this.handleRequestMessage(socket, message)
          break

        default:
          this.logger.warn(`Unknown message type from client`, {
            type: message.type,
            clientId: socket.id
          })
      }
    } catch (error) {
      this.logger.error('Error handling client message', {
        error: error instanceof Error ? error.message : String(error),
        clientId: socket.id
      })
    }
  }

  /**
   * Handle a filter message from a client
   */
  private handleFilterMessage(socket: WebSocketClient, message: any): void {
    // Extract filter options
    const { categories, types, sources } = message

    // Update client filters
    socket.filters = {
      categories: Array.isArray(categories) ? categories : undefined,
      types: Array.isArray(types) ? types : undefined,
      sources: Array.isArray(sources) ? sources : undefined
    }

    // Confirm filters were applied
    socket.send(
      JSON.stringify({
        type: 'filter_response',
        id: generateEventId(),
        timestamp: Date.now(),
        category: EventCategory.SYSTEM,
        source: {
          id: 'websocket-server',
          name: 'WebSocket Server',
          type: 'system'
        },
        data: {
          message: 'Filters applied',
          filters: socket.filters
        }
      })
    )
  }

  /**
   * Handle a ping message from a client
   */
  private handlePingMessage(socket: WebSocketClient): void {
    // Mark client as alive
    socket.isAlive = true

    // Send pong response
    socket.send(
      JSON.stringify({
        type: 'pong',
        id: generateEventId(),
        timestamp: Date.now(),
        category: EventCategory.SYSTEM,
        source: {
          id: 'websocket-server',
          name: 'WebSocket Server',
          type: 'system'
        },
        data: {
          serverTime: Date.now()
        }
      })
    )
  }

  /**
   * Handle a request message from a client
   */
  private handleRequestMessage(socket: WebSocketClient, message: any): void {
    const { requestType, requestId } = message

    // Handle different request types
    switch (requestType) {
      case 'server_status':
        this.sendServerStatus(socket, requestId)
        break

      case 'recent_events':
        this.sendRecentEvents(socket, message.params || {}, requestId)
        break

      default:
        // Send error response for unknown request type
        socket.send(
          JSON.stringify({
            type: 'response',
            id: generateEventId(),
            timestamp: Date.now(),
            category: EventCategory.SYSTEM,
            source: {
              id: 'websocket-server',
              name: 'WebSocket Server',
              type: 'system'
            },
            data: {
              requestId,
              error: `Unknown request type: ${requestType}`,
              success: false
            }
          })
        )
    }
  }

  /**
   * Send server status to a client
   */
  private sendServerStatus(socket: WebSocketClient, requestId: string): void {
    socket.send(
      JSON.stringify({
        type: 'response',
        id: generateEventId(),
        timestamp: Date.now(),
        category: EventCategory.SYSTEM,
        source: {
          id: 'websocket-server',
          name: 'WebSocket Server',
          type: 'system'
        },
        data: {
          requestId,
          success: true,
          status: this.getStatus()
        }
      })
    )
  }

  /**
   * Send recent events to a client
   */
  private sendRecentEvents(socket: WebSocketClient, params: any, requestId: string): void {
    // Get recent events from cache
    const events =
      (this.eventBus as any).getRecentEvents?.({
        limit: params.limit || 20,
        category: params.category,
        type: params.type,
        sourceId: params.sourceId,
        sourceType: params.sourceType,
        since: params.since
      }) || []

    // Send events to client
    socket.send(
      JSON.stringify({
        type: 'response',
        id: generateEventId(),
        timestamp: Date.now(),
        category: EventCategory.SYSTEM,
        source: {
          id: 'websocket-server',
          name: 'WebSocket Server',
          type: 'system'
        },
        data: {
          requestId,
          success: true,
          events,
          count: events.length
        }
      })
    )
  }

  /**
   * Ping all clients to check if they're still alive
   */
  private pingClients(): void {
    this.clients.forEach((socket) => {
      if (socket.isAlive === false) {
        socket.terminate()
        return
      }

      socket.isAlive = false
      socket.ping()
    })
  }

  /**
   * Broadcast an event to all connected clients
   */
  private broadcastEvent(event: BaseEvent): void {
    if (!this.isRunning || this.clients.length === 0) {
      return
    }

    const message = JSON.stringify(event)

    // Send to each client that passes the filter
    for (const client of this.clients) {
      if (client.readyState !== WebSocket.OPEN) {
        continue
      }

      // Apply client-specific filters if any
      if (client.filters) {
        // Check category filter
        if (client.filters.categories && !client.filters.categories.includes(event.category)) {
          continue
        }

        // Check type filter
        if (client.filters.types && !client.filters.types.includes(event.type)) {
          continue
        }

        // Check source filter
        if (client.filters.sources && !client.filters.sources.includes(event.source.id)) {
          continue
        }
      }

      // Send the event
      client.send(message)
    }
  }
}

/**
 * Generate a unique event ID
 */
function generateEventId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substr(2, 5)
}
