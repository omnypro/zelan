import {
  Observable,
  Subject,
  Observer,
  timer,
  EMPTY,
  OperatorFunction,
  MonoTypeOperatorFunction,
} from 'rxjs';
import {
  catchError,
  retry,
  tap,
  finalize,
  timeout,
  takeUntil,
  share,
  switchMap,
} from 'rxjs/operators';

/**
 * Standard timeout duration for operations in milliseconds
 */
export const DEFAULT_TIMEOUT = 30000;

/**
 * Apply timeout operator with a default value
 * 
 * @param duration Timeout duration in milliseconds
 * @returns RxJS operator that applies timeout
 */
export function withTimeout<T>(duration = DEFAULT_TIMEOUT): MonoTypeOperatorFunction<T> {
  return timeout(duration);
}

/**
 * Handle errors in an observable stream
 * 
 * @param errorHandler Function to handle errors
 * @param retryCount Number of retries before failing
 * @returns RxJS operator for error handling
 */
export function handleError<T>(
  errorHandler: (error: unknown) => void,
  retryCount = 3
): MonoTypeOperatorFunction<T> {
  return (source: Observable<T>) =>
    source.pipe(
      retry(retryCount),
      catchError((error) => {
        errorHandler(error);
        return EMPTY;
      })
    );
}

/**
 * Create a logger operator for debugging observable streams
 * 
 * @param prefix Text prefix for the logged messages
 * @returns RxJS operator for logging
 */
export function logEvents<T>(prefix: string): MonoTypeOperatorFunction<T> {
  return tap({
    next: (value) => console.log(`${prefix} - Next:`, value),
    error: (error) => console.error(`${prefix} - Error:`, error),
    complete: () => console.log(`${prefix} - Complete`),
  });
}

/**
 * Exponential backoff retry strategy for reconnecting
 * 
 * @param maxRetries Maximum number of retries
 * @param initialDelay Initial delay in milliseconds
 * @returns RxJS operator for retrying with backoff
 */
export function retryWithBackoff<T>(
  maxRetries = 5,
  initialDelay = 1000
): MonoTypeOperatorFunction<T> {
  return (source: Observable<T>) =>
    source.pipe(
      retry({
        count: maxRetries,
        delay: (error, retryCount) => {
          const delay = Math.pow(2, retryCount) * initialDelay;
          console.log(`Retrying after ${delay}ms, attempt ${retryCount}/${maxRetries}`);
          return timer(delay);
        },
      })
    );
}

/**
 * Take until with finalize for proper cleanup
 * 
 * @param notifier$ Observable that signals when to complete
 * @param finalizeAction Optional cleanup action
 * @returns RxJS operator for managed takeUntil
 */
export function takeUntilWithFinalize<T>(
  notifier$: Observable<any>,
  finalizeAction?: () => void
): MonoTypeOperatorFunction<T> {
  return (source: Observable<T>) =>
    source.pipe(
      takeUntil(notifier$),
      finalize(() => {
        if (finalizeAction) {
          finalizeAction();
        }
      })
    );
}

/**
 * Create a shared observable with proper cleanup
 * 
 * @param factory Factory function that creates the source observable
 * @param cleanup Optional cleanup function
 * @returns Shared observable
 */
export function createSharedObservable<T>(
  factory: () => Observable<T>,
  cleanup?: () => void
): Observable<T> {
  let refCount = 0;
  const subject = new Subject<void>();
  
  return new Observable<T>((observer: Observer<T>) => {
    if (refCount === 0) {
      console.log('Creating shared observable');
    }
    
    refCount++;
    
    const subscription = factory()
      .pipe(
        takeUntil(subject),
        finalize(() => {
          refCount--;
          if (refCount === 0) {
            console.log('Cleaning up shared observable');
            if (cleanup) {
              cleanup();
            }
          }
        }),
        share()
      )
      .subscribe(observer);
    
    return () => {
      subscription.unsubscribe();
      if (refCount === 0) {
        subject.next();
        subject.complete();
      }
    };
  });
}

/**
 * Apply a switchMap that handles errors gracefully
 * 
 * @param project Projection function
 * @param errorHandler Function to handle errors
 * @returns RxJS operator for safe switchMap
 */
export function safeSwitchMap<T, R>(
  project: (value: T) => Observable<R>,
  errorHandler: (error: unknown) => void
): OperatorFunction<T, R> {
  return switchMap((value: T) =>
    project(value).pipe(
      catchError((error) => {
        errorHandler(error);
        return EMPTY;
      })
    )
  );
}