import { useState, useEffect, useRef, useCallback } from 'react'
import { Observable, BehaviorSubject, Subject } from 'rxjs'
import { takeUntil, distinctUntilChanged, map } from 'rxjs/operators'
import { StateContainer } from '@shared/core/utils/stateManagement'
import { DataStream } from '@shared/core/utils/dataStream'

/**
 * React hook for reactive state that integrates with StateContainer from stateManagement.ts
 *
 * @param container State container to connect to
 * @returns [state, update, reset] tuple
 */
export function useStateContainer<T extends object>(
  container: StateContainer<T>
): [T, (partialState: Partial<T>) => void, () => void] {
  const [state, setState] = useState<T>(container.get())

  // Update function
  const update = useCallback(
    (partialState: Partial<T>) => {
      container.update(partialState)
    },
    [container]
  )

  // Reset function
  const reset = useCallback(() => {
    container.reset()
  }, [container])

  useEffect(() => {
    // Subscribe to container state
    const subscription = container.state$.subscribe((newState) => {
      setState(newState)
    })

    // Clean up subscription
    return () => {
      subscription.unsubscribe()
    }
  }, [container])

  return [state, update, reset]
}

/**
 * React hook for a specific slice of a state container
 *
 * @param container State container to connect to
 * @param key Key of the state slice to observe
 * @returns Current value of the state slice
 */
export function useStateSlice<T extends object, K extends keyof T>(
  container: StateContainer<T>,
  key: K
): T[K] {
  const [value, setValue] = useState<T[K]>(container.get()[key])

  useEffect(() => {
    // Subscribe to the specific slice
    const subscription = container.select(key).subscribe((newValue) => {
      setValue(newValue)
    })

    // Clean up subscription
    return () => {
      subscription.unsubscribe()
    }
  }, [container, key])

  return value
}

/**
 * React hook for DataStream integration
 *
 * @param dataStream DataStream to connect to
 * @returns [value, push, update] tuple
 */
export function useDataStream<T>(
  dataStream: DataStream<T>
): [T | undefined, (value: T) => void, (updateFn: (current: T) => T) => void] {
  const [value, setValue] = useState<T | undefined>(dataStream.getValue())

  // Push function
  const push = useCallback(
    (newValue: T) => {
      dataStream.push(newValue)
    },
    [dataStream]
  )

  // Update function
  const update = useCallback(
    (updateFn: (current: T) => T) => {
      dataStream.update(updateFn)
    },
    [dataStream]
  )

  useEffect(() => {
    // Subscribe to data stream
    const subscription = dataStream.stream$.subscribe((newValue) => {
      setValue(newValue)
    })

    // Clean up subscription
    return () => {
      subscription.unsubscribe()
    }
  }, [dataStream])

  return [value, push, update]
}

/**
 * React hook for connecting to any Observable with loading and error states
 *
 * @param source$ Source observable
 * @param initialValue Optional initial value
 * @returns Object with value, loading, and error states
 */
export function useObservableState<T>(
  source$: Observable<T>,
  initialValue?: T
): {
  value: T | undefined
  loading: boolean
  error: Error | null
} {
  const [state, setState] = useState<{
    value: T | undefined
    loading: boolean
    error: Error | null
  }>({
    value: initialValue,
    loading: true,
    error: null
  })

  useEffect(() => {
    // Reset loading state when source changes
    setState((prev) => ({ ...prev, loading: true }))

    // Create a subject for managing the subscription
    const destroy$ = new Subject<void>()

    // Subscribe to the source observable
    const subscription = source$.pipe(takeUntil(destroy$)).subscribe({
      next: (value) => {
        setState({
          value,
          loading: false,
          error: null
        })
      },
      error: (error) => {
        setState({
          value: initialValue,
          loading: false,
          error: error instanceof Error ? error : new Error(String(error))
        })
      }
    })

    // Clean up subscription
    return () => {
      destroy$.next()
      destroy$.complete()
      subscription.unsubscribe()
    }
  }, [source$, initialValue])

  return state
}

/**
 * React hook for creating and managing a BehaviorSubject
 *
 * @param initialValue Initial value for the subject
 * @returns [value, setValue, subject$] tuple
 */
export function useBehaviorSubject<T>(
  initialValue: T
): [T, (value: T) => void, BehaviorSubject<T>] {
  // Create a ref to hold the subject instance
  const subjectRef = useRef<BehaviorSubject<T> | null>(null)

  // Initialize the subject if it doesn't exist
  if (subjectRef.current === null) {
    subjectRef.current = new BehaviorSubject<T>(initialValue)
  }

  // Get the current subject
  const subject$ = subjectRef.current

  // State to track the current value
  const [value, setValue] = useState<T>(initialValue)

  // Function to update the value
  const updateValue = useCallback(
    (newValue: T) => {
      subject$.next(newValue)
    },
    [subject$]
  )

  useEffect(() => {
    // Subscribe to the subject
    const subscription = subject$.subscribe((newValue) => {
      setValue(newValue)
    })

    // Clean up subscription
    return () => {
      subscription.unsubscribe()
    }
  }, [subject$])

  return [value, updateValue, subject$]
}

/**
 * React hook for creating a local reactive state using a BehaviorSubject
 *
 * @param initialState Initial state object
 * @returns [state, updateState, reset, state$] tuple
 */
export function useLocalState<T extends object>(
  initialState: T
): [T, (partialState: Partial<T>) => void, () => void, Observable<T>] {
  // Create a ref to hold the subject instance
  const subjectRef = useRef<BehaviorSubject<T> | null>(null)

  // Initialize the subject if it doesn't exist
  if (subjectRef.current === null) {
    subjectRef.current = new BehaviorSubject<T>(initialState)
  }

  // Get the current subject
  const state$ = subjectRef.current

  // State to track the current value
  const [state, setState] = useState<T>(initialState)

  // Function to update the state
  const updateState = useCallback(
    (partialState: Partial<T>) => {
      const currentState = state$.getValue()
      state$.next({ ...currentState, ...partialState })
    },
    [state$]
  )

  // Function to reset the state
  const resetState = useCallback(() => {
    state$.next(initialState)
  }, [state$, initialState])

  useEffect(() => {
    // Subscribe to the subject
    const subscription = state$.subscribe((newState) => {
      setState(newState)
    })

    // Clean up subscription
    return () => {
      subscription.unsubscribe()
    }
  }, [state$])

  return [state, updateState, resetState, state$.asObservable()]
}

/**
 * React hook for selecting a derived value from an Observable with a selector function
 *
 * @param source$ Source observable
 * @param selectorFn Selector function to derive a value
 * @param initialValue Optional initial value
 * @param equalityFn Optional equality function for distinctUntilChanged
 * @returns Selected value
 */
export function useSelector<T, R>(
  source$: Observable<T>,
  selectorFn: (state: T) => R,
  initialValue?: R,
  equalityFn?: (previous: R, current: R) => boolean
): R | undefined {
  const [value, setValue] = useState<R | undefined>(initialValue)

  useEffect(() => {
    // Create a subject for managing the subscription
    const destroy$ = new Subject<void>()

    // Create derived observable with selector
    const derived$ = source$.pipe(
      takeUntil(destroy$),
      map((value) => selectorFn(value)),
      distinctUntilChanged(equalityFn)
    )

    // Subscribe to the derived observable
    const subscription = derived$.subscribe({
      next: (selectedValue) => {
        setValue(selectedValue)
      },
      error: (error) => {
        console.error('Error in selector:', error)
      }
    })

    // Clean up subscription
    return () => {
      destroy$.next()
      destroy$.complete()
      subscription.unsubscribe()
    }
  }, [source$, selectorFn, equalityFn])

  return value
}
