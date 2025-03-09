import { WebSocketServer, WebSocketServerConfig } from './WebSocketServer'
import { MainEventBus } from '../eventBus'
import { SystemEventType, EventCategory } from '@s/types/events'

/**
 * Service that manages the WebSocket server
 */
export class WebSocketService {
  private server: WebSocketServer
  private static instance: WebSocketService

  private constructor(
    private eventBus: MainEventBus,
    config?: Partial<WebSocketServerConfig>
  ) {
    this.server = new WebSocketServer(eventBus, config)
  }

  /**
   * Get the WebSocketService singleton instance
   */
  static getInstance(
    eventBus: MainEventBus,
    config?: Partial<WebSocketServerConfig>
  ): WebSocketService {
    if (!WebSocketService.instance) {
      WebSocketService.instance = new WebSocketService(eventBus, config)
    }
    return WebSocketService.instance
  }

  /**
   * Start the WebSocket server
   */
  start(): boolean {
    const result = this.server.start()

    if (result) {
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
        payload: {
          message: `WebSocket server started on port ${this.server.getStatus().port}`
        },
        data: {
          message: `WebSocket server started on port ${this.server.getStatus().port}`
        },
        metadata: {
          version: '1.0'
        }
      })
    }

    return result
  }

  /**
   * Stop the WebSocket server
   */
  stop(): void {
    this.server.stop()

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
      payload: {
        message: 'WebSocket server stopped'
      },
      data: {
        message: 'WebSocket server stopped'
      },
      metadata: {
        version: '1.0'
      }
    })
  }

  /**
   * Get the server status
   */
  getStatus(): { running: boolean; clientCount: number; port: number } {
    return this.server.getStatus()
  }
}
