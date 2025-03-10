import { useState, useEffect } from 'react'
import { Observable, isObservable } from 'rxjs'

/**
 * Hook result with status information
 */
export interface ObservableState<T> {
  value: T
  loading: boolean
  error: Error | null
  completed: boolean
}

/**
 * Core hook for working with observables in React components
 */
export function useObservableState<T>(source$: Observable<T> | null | undefined, initialValue?: T): ObservableState<T> {
  const [state, setState] = useState<ObservableState<T>>({
    value: initialValue as T,
    loading: Boolean(source$),
    error: null,
    completed: false
  })

  useEffect(() => {
    // No source observable provided
    if (!isObservable(source$)) {
      setState({
        value: initialValue as T,
        loading: false,
        error: source$ === undefined || source$ === null ? null : new Error('Invalid observable provided'),
        completed: false
      })
      return () => {}
    }

    // Subscribe to the observable
    const subscription = source$.subscribe({
      next: (value: T) =>
        setState({
          value,
          loading: false,
          error: null,
          completed: false
        }),
      error: (error: Error) =>
        setState((prev) => ({
          ...prev,
          loading: false,
          error,
          completed: true
        })),
      complete: () =>
        setState((prev) => ({
          ...prev,
          loading: false,
          completed: true
        }))
    })

    // Clean up subscription when component unmounts or source changes
    return () => subscription.unsubscribe()
  }, [source$, initialValue])

  return state
}

/**
 * Simple hook that returns just the value from an observable
 */
export function useObservable<T>(source$: Observable<T> | null | undefined, initialValue?: T): T {
  const { value } = useObservableState(source$, initialValue)
  return value
}
