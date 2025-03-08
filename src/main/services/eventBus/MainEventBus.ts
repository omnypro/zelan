import { ipcMain } from 'electron'
import { BaseEventBus } from '@s/core/bus'
import { BaseEvent } from '@s/types/events'

// IPC channels
const EVENT_CHANNEL = 'zelan:event'
const EVENT_SYNC_CHANNEL = 'zelan:event-sync'

/**
 * Main process event bus with IPC support
 */
export class MainEventBus extends BaseEventBus {
  private webContents: Electron.WebContents[] = []
  private isForwarding = false

  constructor() {
    super()

    // Set up IPC listeners
    this.setupIpcListeners()
  }

  /**
   * Add a WebContents instance to forward events to
   */
  addWebContents(webContents: Electron.WebContents): void {
    if (!this.webContents.includes(webContents)) {
      this.webContents.push(webContents)

      // Remove when destroyed
      webContents.on('destroyed', () => {
        this.removeWebContents(webContents)
      })
    }
  }

  /**
   * Remove a WebContents instance
   */
  removeWebContents(webContents: Electron.WebContents): void {
    const index = this.webContents.indexOf(webContents)
    if (index !== -1) {
      this.webContents.splice(index, 1)
    }
  }

  /**
   * Publish an event to the event bus
   * Will also forward to renderer processes
   */
  publish<T>(event: BaseEvent<T>): void {
    super.publish(event)

    // Avoid recursive forwarding
    if (!this.isForwarding) {
      this.forwardToRenderers(event)
    }
  }

  /**
   * Set up IPC listeners for events from renderer processes
   */
  private setupIpcListeners(): void {
    // Handle async events from renderer
    ipcMain.on(EVENT_CHANNEL, (event, serializedEvent) => {
      try {
        // Mark as forwarding to prevent loops
        this.isForwarding = true

        // Publish to main process event bus
        super.publish(serializedEvent)

        // Forward to all other renderers except sender
        this.forwardToRenderers(serializedEvent, event.sender)

        this.isForwarding = false
      } catch (error) {
        console.error('Error handling event from renderer:', error)
        this.isForwarding = false
      }
    })

    // Handle sync events from renderer
    ipcMain.on(EVENT_SYNC_CHANNEL, (event, serializedEvent) => {
      try {
        // Mark as forwarding to prevent loops
        this.isForwarding = true

        // Publish to main process event bus
        super.publish(serializedEvent)

        // Forward to all other renderers except sender
        this.forwardToRenderers(serializedEvent, event.sender)

        this.isForwarding = false

        // Acknowledge receipt
        event.returnValue = { success: true }
      } catch (error) {
        console.error('Error handling sync event from renderer:', error)
        this.isForwarding = false
        event.returnValue = { success: false, error: String(error) }
      }
    })
  }

  /**
   * Forward an event to all renderer processes
   */
  private forwardToRenderers<T>(event: BaseEvent<T>, excludeSender?: Electron.WebContents): void {
    // Send to all registered WebContents
    for (const contents of this.webContents) {
      if (contents !== excludeSender && !contents.isDestroyed()) {
        contents.send(EVENT_CHANNEL, event)
      }
    }
  }
}
