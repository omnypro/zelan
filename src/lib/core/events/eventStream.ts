import { Observable, Subject, takeUntil, filter, share, map } from 'rxjs';
import { BaseEvent } from '@shared/types';
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
  public pipe<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R>(
    op1: import('rxjs').OperatorFunction<T, A>,
    op2?: import('rxjs').OperatorFunction<A, B>,
    op3?: import('rxjs').OperatorFunction<B, C>,
    op4?: import('rxjs').OperatorFunction<C, D>,
    op5?: import('rxjs').OperatorFunction<D, E>,
    op6?: import('rxjs').OperatorFunction<E, F>,
    op7?: import('rxjs').OperatorFunction<F, G>,
    op8?: import('rxjs').OperatorFunction<G, H>,
    op9?: import('rxjs').OperatorFunction<H, I>,
    op10?: import('rxjs').OperatorFunction<I, J>,
    op11?: import('rxjs').OperatorFunction<J, K>,
    op12?: import('rxjs').OperatorFunction<K, L>,
    op13?: import('rxjs').OperatorFunction<L, M>,
    op14?: import('rxjs').OperatorFunction<M, N>,
    op15?: import('rxjs').OperatorFunction<N, O>,
    op16?: import('rxjs').OperatorFunction<O, P>,
    op17?: import('rxjs').OperatorFunction<P, Q>,
    op18?: import('rxjs').OperatorFunction<Q, R>,
  ): Observable<R> {
    const args = [op1, op2, op3, op4, op5, op6, op7, op8, op9, op10, op11, op12, op13, op14, op15, op16, op17, op18].filter(Boolean);
    return this.stream$.pipe.apply(this.stream$, args as any) as Observable<R>;
  }
  
  /**
   * Clean up subscriptions
   */
  public destroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }
}