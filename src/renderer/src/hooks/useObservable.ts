import { useState, useEffect } from 'react';
import { Observable } from 'rxjs';

/**
 * React hook for subscribing to an Observable
 * 
 * @param observable$ Observable to subscribe to
 * @param initialValue Initial value before the observable emits
 * @returns Current value from the observable
 */
export function useObservable<T>(observable$: Observable<T>, initialValue: T): T {
  const [value, setValue] = useState<T>(initialValue);

  useEffect(() => {
    const subscription = observable$.subscribe({
      next: (value) => setValue(value),
      error: (error) => console.error('Error in observable:', error),
    });

    return () => {
      subscription.unsubscribe();
    };
  }, [observable$]);

  return value;
}

/**
 * React hook for subscribing to an Observable and getting loading state
 * 
 * @param observable$ Observable to subscribe to
 * @param initialValue Initial value before the observable emits
 * @returns Object with current value, loading state, and error
 */
export function useObservableWithStatus<T>(
  observable$: Observable<T>,
  initialValue?: T
): {
  value: T | undefined;
  loading: boolean;
  error: Error | null;
} {
  const [state, setState] = useState<{
    value: T | undefined;
    loading: boolean;
    error: Error | null;
  }>({
    value: initialValue,
    loading: true,
    error: null,
  });

  useEffect(() => {
    setState((prev) => ({ ...prev, loading: true }));

    const subscription = observable$.subscribe({
      next: (value) => {
        setState({
          value,
          loading: false,
          error: null,
        });
      },
      error: (error) => {
        setState({
          value: initialValue,
          loading: false,
          error: error instanceof Error ? error : new Error(String(error)),
        });
      },
    });

    return () => {
      subscription.unsubscribe();
    };
  }, [observable$, initialValue]);

  return state;
}

export default useObservable;