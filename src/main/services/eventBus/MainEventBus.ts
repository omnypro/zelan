import { ipcMain, IpcMainEvent, webContents } from 'electron';
import { EventBus } from '../../../shared/core/bus/EventBus';
import { Event } from '../../../shared/types/events';
import { BaseEvent } from '../../../shared/core/events/BaseEvent';

/**
 * IPC channel names for event communication
 */
export const IPC_CHANNELS = {
  PUBLISH_EVENT: 'eventBus:publish',
  SUBSCRIBE_EVENTS: 'eventBus:subscribe',
  REQUEST_EVENTS: 'eventBus:requestEvents',
  RECEIVE_EVENTS: 'eventBus:receiveEvents',
};

/**
 * Main process implementation of the event bus
 * Handles IPC communication with renderer processes
 */
export class MainEventBus extends EventBus {
  /**
   * Flag to prevent recursive event publication
   */
  private isForwarding = false;

  /**
   * Create a new main process event bus
   */
  constructor() {
    super();
    this.setupIpcHandlers();
  }

  /**
   * Set up IPC handlers for event communication
   */
  private setupIpcHandlers(): void {
    // Handle events published from renderer
    ipcMain.on(IPC_CHANNELS.PUBLISH_EVENT, (_: IpcMainEvent, serializedEvent: string) => {
      try {
        const deserialized = BaseEvent.deserialize(serializedEvent);
        this.forwardToEventBus(deserialized);
      } catch (error) {
        console.error('Error deserializing event from renderer:', error);
      }
    });

    // Handle event subscription requests from renderer
    ipcMain.handle(IPC_CHANNELS.REQUEST_EVENTS, async () => {
      return {
        success: true,
        message: 'Event stream established',
      };
    });

    // Subscribe to all events to forward them to renderers
    this.subscribe((event) => {
      if (!this.isForwarding) {
        this.forwardToRenderers(event);
      }
    });
  }

  /**
   * Forward an event from renderer to the event bus
   * 
   * @param event Event to forward
   */
  private forwardToEventBus(event: Event): void {
    // Prevent recursive publishing
    this.isForwarding = true;
    try {
      this.publish(event);
    } finally {
      this.isForwarding = false;
    }
  }

  /**
   * Forward an event from the event bus to all renderer processes
   * 
   * @param event Event to forward
   */
  private forwardToRenderers(event: Event): void {
    try {
      // Convert the event to a serialized form
      const serializedEvent = (event instanceof BaseEvent)
        ? event.serialize()
        : JSON.stringify(event);

      // Send to all renderer processes
      webContents.getAllWebContents().forEach((contents) => {
        contents.send(IPC_CHANNELS.RECEIVE_EVENTS, serializedEvent);
      });
    } catch (error) {
      console.error('Error forwarding event to renderers:', error);
    }
  }
}