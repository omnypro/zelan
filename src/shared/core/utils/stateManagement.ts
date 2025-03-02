import {
  BehaviorSubject,
  Observable,
  Subject,
  distinctUntilChanged,
  map,
  shareReplay,
  tap
} from 'rxjs'

/**
 * Interface for state containers
 */
export interface StateContainer<T> {
  /**
   * Get the current state value
   */
  get(): T

  /**
   * Update the state with a partial state
   */
  update(partialState: Partial<T>): void

  /**
   * Update the state with a function that takes current state and returns new state
   */
  updateWithFn(updateFn: (state: T) => T): void

  /**
   * Reset the state to initial value
   */
  reset(): void

  /**
   * Observable of the state changes
   */
  state$: Observable<T>

  /**
   * Get observable for a specific slice of the state
   */
  select<K extends keyof T>(key: K): Observable<T[K]>

  /**
   * Get observable with a selector function
   */
  selectWithFn<R>(selectorFn: (state: T) => R): Observable<R>
}

/**
 * Create a state container with BehaviorSubject
 *
 * @param initialState Initial state
 * @returns State container
 */
export function createStateContainer<T extends object>(initialState: T): StateContainer<T> {
  const state$ = new BehaviorSubject<T>(initialState)

  // Create a shared replay observable for better performance
  const sharedState$ = state$.pipe(shareReplay(1))

  return {
    get: () => state$.getValue(),

    update: (partialState: Partial<T>) => {
      const currentState = state$.getValue()
      state$.next({ ...currentState, ...partialState })
    },

    updateWithFn: (updateFn: (state: T) => T) => {
      const currentState = state$.getValue()
      state$.next(updateFn(currentState))
    },

    reset: () => {
      state$.next(initialState)
    },

    state$: sharedState$,

    select: <K extends keyof T>(key: K): Observable<T[K]> => {
      return sharedState$.pipe(
        map((state) => state[key]),
        distinctUntilChanged()
      )
    },

    selectWithFn: <R>(selectorFn: (state: T) => R): Observable<R> => {
      return sharedState$.pipe(map(selectorFn), distinctUntilChanged())
    }
  }
}

/**
 * Create actions for a state container
 *
 * @param container State container to attach actions to
 * @returns Object with action subjects and action creators
 */
export function createActions<T extends object, A extends Record<string, any>>(
  container: StateContainer<T>,
  actionHandlers: {
    [K in keyof A]: (state: T, payload: A[K]) => Partial<T> | T
  }
) {
  const actionSubjects: Record<string, Subject<any>> = {}
  const actionCreators: Record<string, (payload: any) => void> = {}

  // Create subjects and action creators for each handler
  for (const actionType in actionHandlers) {
    const subject = new Subject<any>()

    // Add subject to record
    actionSubjects[actionType] = subject

    // Create action creator
    actionCreators[actionType] = (payload: any) => {
      subject.next(payload)
    }

    // Subscribe to subject to update state
    subject.subscribe((payload) => {
      const handler = actionHandlers[actionType]
      const currentState = container.get()
      const result = handler(currentState, payload)

      // If handler returns full state object, replace completely
      if (Object.keys(result).length === Object.keys(currentState).length) {
        container.updateWithFn(() => result as T)
      } else {
        // Otherwise treat as partial state update
        container.update(result)
      }
    })
  }

  return {
    subjects: actionSubjects as { [K in keyof A]: Subject<A[K]> },
    actions: actionCreators as { [K in keyof A]: (payload: A[K]) => void }
  }
}

/**
 * Create selectors for a state container
 *
 * @param container State container to create selectors for
 * @param selectors Object of selector functions
 * @returns Object with selector observables
 */
export function createSelectors<T extends object, S extends Record<string, (state: T) => any>>(
  container: StateContainer<T>,
  selectors: S
): { [K in keyof S]: Observable<ReturnType<S[K]>> } {
  const selectorObservables: Record<string, Observable<any>> = {}

  for (const key in selectors) {
    selectorObservables[key] = container.selectWithFn(selectors[key])
  }

  return selectorObservables as { [K in keyof S]: Observable<ReturnType<S[K]>> }
}

/**
 * Create a debug observer for a state container
 *
 * @param container State container to debug
 * @param name Optional name for the container
 * @returns Function to enable/disable debugging
 */
export function debugState<T>(
  container: StateContainer<T>,
  name: string = 'StateContainer'
): () => void {
  const subscription = container.state$
    .pipe(tap((state) => console.log(`[${name}] State updated:`, state)))
    .subscribe()

  return () => subscription.unsubscribe()
}
