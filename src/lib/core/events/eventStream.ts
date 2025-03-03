import { Observable, Subject, takeUntil, filter, share, map } from 'rxjs';
import { BaseEvent } from './types';
import { EventBus } from './eventBus';

/**
 * EventStream provides a higher-level abstraction over the EventBus
 * for creating specialized event streams with transformation capabilities.
 */
export class EventStream<T extends BaseEvent> {
  private destroy$ = new Subject<void>();
  private eventBus: EventBus;
  private stream$: Observable<T>;
  
  /**
   * Create a new event stream with optional filtering
   */
  constructor(
    typeOrFilter?: string | ((event: BaseEvent) => boolean),
    source?: string
  ) {
    this.eventBus = EventBus.getInstance();
    
    // Start with the full event stream
    let baseStream$ = this.eventBus.events() as Observable<T>;
    
    // Apply type filter if provided
    if (typeof typeOrFilter === 'string') {
      baseStream$ = baseStream$.pipe(
        filter((event): event is T => event.type === typeOrFilter)
      );
    } 
    // Apply custom filter if provided
    else if (typeof typeOrFilter === 'function') {
      baseStream$ = baseStream$.pipe(
        filter((event): event is T => typeOrFilter(event))
      );
    }
    
    // Apply source filter if provided
    if (source) {
      baseStream$ = baseStream$.pipe(
        filter((event): event is T => event.source === source)
      );
    }
    
    // Share the stream to avoid multiple subscriptions triggering multiple times
    this.stream$ = baseStream$.pipe(
      takeUntil(this.destroy$),
      share()
    );
  }
  
  /**
   * Get the observable stream
   */
  public get stream(): Observable<T> {
    return this.stream$;
  }
  
  /**
   * Map events to another type
   */
  public map<R>(mapFn: (event: T) => R): Observable<R> {
    return this.stream$.pipe(map(mapFn));
  }
  
  /**
   * Apply a custom transformation pipeline
   */
  public pipe<R>(operators: any[]): Observable<R> {
    return this.stream$.pipe.apply(this.stream$, operators);
  }
  
  /**
   * Clean up subscriptions
   */
  public destroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }
}