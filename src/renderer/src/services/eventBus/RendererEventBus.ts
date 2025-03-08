import { BaseEventBus } from '@s/core/bus'
import { BaseEvent } from '@s/types/events'

/**
 * Renderer process event bus with IPC bridge
 */
export class RendererEventBus extends BaseEventBus {
  private isForwarding = false
  private unsubscribeFn: (() => void) | null = null

  constructor() {
    super()

    // Set up IPC bridge
    this.setupIpcBridge()
  }

  /**
   * Publish an event to the event bus
   * Will also forward to main process
   */
  publish<T>(event: BaseEvent<T>): void {
    super.publish(event)

    // Avoid recursive forwarding
    if (!this.isForwarding) {
      this.forwardToMain(event)
    }
  }

  /**
   * Set up IPC bridge to communicate with main process
   */
  private setupIpcBridge(): void {
    // Listen for events from main process
    this.unsubscribeFn = window.api.events.onEvent((event) => {
      try {
        // Mark as forwarding to prevent loops
        this.isForwarding = true

        // Publish to renderer event bus
        super.publish(event)

        this.isForwarding = false
      } catch (error) {
        console.error('Error handling event from main process:', error)
        this.isForwarding = false
      }
    })
  }

  /**
   * Forward an event to the main process
   */
  private forwardToMain<T>(event: BaseEvent<T>): void {
    window.api.events.sendEvent(event)
  }

  /**
   * Clean up event listeners
   */
  dispose(): void {
    if (this.unsubscribeFn) {
      this.unsubscribeFn()
      this.unsubscribeFn = null
    }
  }
}
