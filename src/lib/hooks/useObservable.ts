import { useEffect, useState } from 'react';
import { Observable } from 'rxjs';

/**
 * Hook to subscribe to an Observable and get its emitted values
 * Automatically handles subscription lifecycle
 */
export function useObservable<T>(
  observable: Observable<T>,
  initialValue: T
): T {
  const [value, setValue] = useState<T>(initialValue);

  useEffect(() => {
    const subscription = observable.subscribe(newValue => {
      setValue(newValue);
    });

    // Clean up subscription on unmount
    return () => {
      subscription.unsubscribe();
    };
  }, [observable]);

  return value;
}