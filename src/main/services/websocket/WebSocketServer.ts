import { WebSocket, WebSocketServer as WSServer } from 'ws';
import { Subscription } from 'rxjs';
import { EventBus } from '../../../shared/core/bus';
import { BaseEvent, EventCategory, SystemEventType } from '../../../shared/types/events';

/**
 * WebSocket server configuration
 */
export interface WebSocketServerConfig {
  port: number;
  pingInterval: number; // in milliseconds
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: WebSocketServerConfig = {
  port: 8081,
  pingInterval: 30000 // 30 seconds
};

/**
 * WebSocket server for exposing events to external clients
 */
export class WebSocketServer {
  private server: WSServer | null = null;
  private clients: Set<WebSocket> = new Set();
  private eventSubscription: Subscription | null = null;
  private pingInterval: NodeJS.Timeout | null = null;
  private config: WebSocketServerConfig;
  private isRunning = false;

  constructor(private eventBus: EventBus, config?: Partial<WebSocketServerConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Start the WebSocket server
   * @returns true if started successfully, false if already running
   */
  start(): boolean {
    if (this.isRunning) {
      console.log('WebSocket server is already running');
      return false;
    }

    try {
      // Create WebSocket server
      this.server = new WSServer({ port: this.config.port });
      
      // Set up connection handling
      this.server.on('connection', this.handleConnection.bind(this));
      this.server.on('error', this.handleServerError.bind(this));

      // Subscribe to all events
      this.eventSubscription = this.eventBus.getEvents().subscribe(
        this.broadcastEvent.bind(this)
      );

      // Set up ping interval
      this.setupPingInterval();

      this.isRunning = true;
      console.log(`WebSocket server started on port ${this.config.port}`);
      return true;
    } catch (error) {
      console.error('Failed to start WebSocket server:', error);
      this.stop();
      return false;
    }
  }

  /**
   * Stop the WebSocket server
   */
  stop(): void {
    // Clear ping interval
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }

    // Unsubscribe from events
    if (this.eventSubscription) {
      this.eventSubscription.unsubscribe();
      this.eventSubscription = null;
    }

    // Close all client connections
    for (const client of this.clients) {
      try {
        client.terminate();
      } catch (e) {
        // Ignore errors when closing clients
      }
    }
    this.clients.clear();

    // Close server
    if (this.server) {
      try {
        this.server.close();
      } catch (e) {
        // Ignore errors when closing server
      }
      this.server = null;
    }

    this.isRunning = false;
    console.log('WebSocket server stopped');
  }

  /**
   * Get the server status
   */
  getStatus(): { running: boolean; clientCount: number; port: number } {
    return {
      running: this.isRunning,
      clientCount: this.clients.size,
      port: this.config.port
    };
  }

  /**
   * Set up ping interval to keep connections alive
   */
  private setupPingInterval(): void {
    this.pingInterval = setInterval(() => {
      this.pingClients();
    }, this.config.pingInterval);
  }

  /**
   * Send ping to all clients to keep connections alive
   */
  private pingClients(): void {
    const deadClients: WebSocket[] = [];

    for (const client of this.clients) {
      if (client.readyState === WebSocket.OPEN) {
        try {
          client.ping();
        } catch (e) {
          deadClients.push(client);
        }
      } else if (client.readyState !== WebSocket.CONNECTING) {
        deadClients.push(client);
      }
    }

    // Remove dead clients
    for (const client of deadClients) {
      this.removeClient(client);
    }
  }

  /**
   * Handle new WebSocket connection
   */
  private handleConnection(client: WebSocket): void {
    // Add to client set
    this.clients.add(client);

    // Setup client event listeners
    client.on('close', () => this.removeClient(client));
    client.on('error', () => this.removeClient(client));
    client.on('pong', () => {}); // Keep alive response

    // Send welcome message
    this.sendToClient(client, {
      id: 'welcome',
      timestamp: Date.now(),
      source: 'websocket-server',
      category: EventCategory.SYSTEM,
      type: SystemEventType.INFO,
      payload: {
        message: 'Connected to Zelan WebSocket Server',
        clientCount: this.clients.size
      }
    });

    console.log(`WebSocket client connected. Total clients: ${this.clients.size}`);
  }

  /**
   * Remove a client from the set
   */
  private removeClient(client: WebSocket): void {
    if (this.clients.has(client)) {
      try {
        client.terminate();
      } catch (e) {
        // Ignore errors when terminating
      }
      this.clients.delete(client);
      console.log(`WebSocket client disconnected. Total clients: ${this.clients.size}`);
    }
  }

  /**
   * Handle server error
   */
  private handleServerError(error: Error): void {
    console.error('WebSocket server error:', error);
  }

  /**
   * Broadcast an event to all connected clients
   */
  private broadcastEvent<T>(event: BaseEvent<T>): void {
    if (!this.isRunning || this.clients.size === 0) {
      return;
    }

    const deadClients: WebSocket[] = [];

    for (const client of this.clients) {
      try {
        if (client.readyState === WebSocket.OPEN) {
          this.sendToClient(client, event);
        } else if (client.readyState === WebSocket.CLOSED || client.readyState === WebSocket.CLOSING) {
          deadClients.push(client);
        }
      } catch (e) {
        deadClients.push(client);
      }
    }

    // Clean up dead clients
    for (const client of deadClients) {
      this.removeClient(client);
    }
  }

  /**
   * Send event to a specific client
   */
  private sendToClient<T>(client: WebSocket, event: BaseEvent<T>): void {
    try {
      // Stringify with proper error handling for circular references
      const message = JSON.stringify(event, this.safeReplacer);
      client.send(message);
    } catch (error) {
      console.error('Error sending message to client:', error);
    }
  }

  /**
   * Safe JSON replacer function to handle circular references
   */
  private safeReplacer(_: string, value: any): any {
    // Handle special cases like functions, circular refs, etc.
    if (typeof value === 'function') {
      return '[Function]';
    }
    return value;
  }
}