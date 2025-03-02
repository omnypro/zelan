import { useState, useEffect } from 'react'

// Simple Observable interface that matches our preload interface
interface SimpleObservable<T> {
  subscribe: (callback: (value: T) => void) => () => void
}

/**
 * React hook for subscribing to a simple observable
 *
 * @param observable Observable to subscribe to
 * @param initialValue Initial value before the observable emits
 * @returns Current value from the observable
 */
export function useSimpleObservable<T>(observable: SimpleObservable<T>, initialValue: T): T {
  const [value, setValue] = useState<T>(initialValue)

  useEffect(() => {
    // Subscribe to the observable
    const unsubscribe = observable.subscribe((newValue) => {
      setValue(newValue)
    })

    // Clean up subscription
    return () => {
      unsubscribe()
    }
  }, [observable])

  return value
}

export default useSimpleObservable
