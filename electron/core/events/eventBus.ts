import { Subject, Observable, filter, share } from 'rxjs';
import { BaseEvent, BaseEventSchema } from '@shared/types';

/**
 * EventBus - Central messaging system for the main process
 * 
 * Provides a reactive pub/sub pattern using RxJS for event-driven architecture.
 * All events are validated through Zod schemas to ensure type safety.
 */
export class EventBus {
  private static instance: EventBus;
  private eventSubject: Subject<BaseEvent>;
  
  private constructor() {
    this.eventSubject = new Subject<BaseEvent>();
  }
  
  /**
   * Get singleton instance of EventBus
   */
  public static getInstance(): EventBus {
    if (!EventBus.instance) {
      EventBus.instance = new EventBus();
    }
    return EventBus.instance;
  }
  
  /**
   * Publish an event to the event bus
   */
  public publish(event: BaseEvent): void {
    try {
      // Validate event with schema
      const validatedEvent = BaseEventSchema.parse(event);
      this.eventSubject.next(validatedEvent);
    } catch (error) {
      console.error('Invalid event format:', error);
    }
  }
  
  /**
   * Get all events as an observable
   */
  public events(): Observable<BaseEvent> {
    return this.eventSubject.asObservable().pipe(share());
  }
  
  /**
   * Get events of a specific type
   */
  public ofType<T extends BaseEvent>(type: string): Observable<T> {
    return this.eventSubject.pipe(
      filter((event): event is T => event.type === type),
      share()
    );
  }
  
  /**
   * Get events from a specific source
   */
  public fromSource<T extends BaseEvent>(source: string): Observable<T> {
    return this.eventSubject.pipe(
      filter((event): event is T => event.source === source),
      share()
    );
  }
}