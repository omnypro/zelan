import { useState, useEffect } from 'react';
import { Observable } from 'rxjs';

/**
 * React hook to subscribe to an RxJS Observable
 *
 * @param observable$ The observable to subscribe to
 * @param initialValue Optional initial value
 * @returns The current value of the observable
 */
export function useObservable<T>(observable$: Observable<T> | null | undefined, initialValue?: T): T {
  const [value, setValue] = useState<T>(initialValue as T);

  useEffect(() => {
    // Check if we have a valid observable that can be subscribed to
    if (observable$ && typeof observable$.subscribe === 'function') {
      const subscription = observable$.subscribe({
        next: (value: T) => setValue(value)
      });

      return () => subscription.unsubscribe();
    } else {
      console.warn('Invalid observable passed to useObservable', observable$);
      // Return a no-op cleanup function
      return () => {};
    }
  }, [observable$]);

  return value;
}

/**
 * Hook result with status information
 */
export interface ObservableStatus<T> {
  value: T;
  loading: boolean;
  error: Error | null;
}

/**
 * React hook to subscribe to an RxJS Observable with status information
 *
 * @param observable$ The observable to subscribe to
 * @param initialValue Optional initial value
 * @returns Object with value, loading, and error state
 */
export function useObservableWithStatus<T>(
  observable$: Observable<T> | null | undefined,
  initialValue?: T
): ObservableStatus<T> {
  const [state, setState] = useState<ObservableStatus<T>>({
    value: initialValue as T,
    loading: true,
    error: null
  });

  useEffect(() => {
    // Check if we have a valid observable that can be subscribed to
    if (observable$ && typeof observable$.subscribe === 'function') {
      const subscription = observable$.subscribe({
        next: (value: T) => setState({ value, loading: false, error: null }),
        error: (error: Error) => setState((state) => ({ ...state, loading: false, error }))
      });

      return () => subscription.unsubscribe();
    } else {
      console.warn('Invalid observable passed to useObservableWithStatus', observable$);
      setState({
        value: initialValue as T,
        loading: false,
        error: new Error('Invalid observable provided')
      });
      // Return a no-op cleanup function
      return () => {};
    }
  }, [observable$]);

  return state;
}