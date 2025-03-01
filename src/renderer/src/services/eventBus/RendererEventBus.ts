import { EventBus } from '../../../../shared/core/bus/EventBus';
import { Event } from '../../../../shared/types/events';
import { BaseEvent } from '../../../../shared/core/events/BaseEvent';

/**
 * IPC channel names for event communication
 * Must match the ones defined in MainEventBus
 */
export const IPC_CHANNELS = {
  PUBLISH_EVENT: 'eventBus:publish',
  SUBSCRIBE_EVENTS: 'eventBus:subscribe',
  REQUEST_EVENTS: 'eventBus:requestEvents',
  RECEIVE_EVENTS: 'eventBus:receiveEvents',
};

/**
 * Renderer process implementation of the event bus
 * Communicates with the main process via IPC
 */
export class RendererEventBus extends EventBus {
  /**
   * Flag to prevent recursive event publication
   */
  private isForwarding = false;

  /**
   * Flag to track IPC connection status
   */
  private isConnected = false;

  /**
   * Create a new renderer process event bus
   */
  constructor() {
    super();
    this.setupIpcHandlers();
    this.connectToMainEventBus();
  }

  /**
   * Connect to the main process event bus
   */
  private async connectToMainEventBus(): Promise<void> {
    try {
      const response = await window.electron.ipcRenderer.invoke(IPC_CHANNELS.REQUEST_EVENTS);
      this.isConnected = response.success;
      console.log('Connected to main process event bus:', response.message);
    } catch (error) {
      console.error('Failed to connect to main process event bus:', error);
      this.isConnected = false;
      
      // Retry connection after a delay
      setTimeout(() => {
        this.connectToMainEventBus();
      }, 5000);
    }
  }

  /**
   * Set up IPC handlers for event communication
   */
  private setupIpcHandlers(): void {
    // Listen for events from the main process
    window.electron.ipcRenderer.on(IPC_CHANNELS.RECEIVE_EVENTS, (_event, serializedEvent) => {
      try {
        const deserialized = BaseEvent.deserialize(serializedEvent);
        this.forwardToEventBus(deserialized);
      } catch (error) {
        console.error('Error deserializing event from main process:', error);
      }
    });
  }

  /**
   * Override publish to also send events to the main process
   * 
   * @param event Event to publish
   */
  public override publish<T>(event: Event<T>): void {
    // Call the parent implementation to publish locally
    super.publish(event);
    
    // Forward to the main process if not already forwarding
    if (!this.isForwarding && this.isConnected) {
      this.forwardToMain(event);
    }
  }

  /**
   * Forward an event from main to the event bus
   * 
   * @param event Event to forward
   */
  private forwardToEventBus(event: Event): void {
    // Prevent recursive publishing
    this.isForwarding = true;
    try {
      super.publish(event);
    } finally {
      this.isForwarding = false;
    }
  }

  /**
   * Forward an event from the event bus to the main process
   * 
   * @param event Event to forward
   */
  private forwardToMain(event: Event): void {
    try {
      // Convert the event to a serialized form
      const serializedEvent = (event instanceof BaseEvent)
        ? event.serialize()
        : JSON.stringify(event);
      
      // Send to the main process
      window.electron.ipcRenderer.send(IPC_CHANNELS.PUBLISH_EVENT, serializedEvent);
    } catch (error) {
      console.error('Error forwarding event to main process:', error);
    }
  }
}