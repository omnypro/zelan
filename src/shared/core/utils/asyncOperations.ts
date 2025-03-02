import {
  Observable,
  of,
  from,
  throwError,
  Subject,
  BehaviorSubject,
  timer,
  defer,
  merge,
  OperatorFunction,
  take,
  scan
} from 'rxjs'
import {
  catchError,
  map,
  tap,
  switchMap,
  mergeMap,
  concatMap,
  timeout,
  takeUntil,
  filter,
  shareReplay,
  distinctUntilChanged,
  retryWhen
} from 'rxjs/operators'

/**
 * Status of an asynchronous operation
 */
export enum AsyncStatus {
  IDLE = 'idle',
  PENDING = 'pending',
  SUCCESS = 'success',
  ERROR = 'error'
}

/**
 * Result of an asynchronous operation
 */
export interface AsyncResult<T, E = Error> {
  /**
   * Current status of the operation
   */
  status: AsyncStatus

  /**
   * Result value (if status is SUCCESS)
   */
  data: T | null

  /**
   * Error information (if status is ERROR)
   */
  error: E | null

  /**
   * Timestamp when the operation started
   */
  startTime: number | null

  /**
   * Timestamp when the operation completed
   */
  endTime: number | null

  /**
   * Whether the operation is currently in progress
   */
  isPending: boolean

  /**
   * Whether the operation completed successfully
   */
  isSuccess: boolean

  /**
   * Whether the operation completed with an error
   */
  isError: boolean
}

/**
 * Options for async operation execution
 */
export interface AsyncOperationOptions<T, E = Error> {
  /**
   * Operation name for debugging
   */
  name?: string

  /**
   * Function to handle successful results
   */
  onSuccess?: (result: T) => void

  /**
   * Function to handle errors
   */
  onError?: (error: E) => void

  /**
   * Whether to automatically retry on error
   */
  retry?: boolean

  /**
   * Number of retry attempts
   */
  retryCount?: number

  /**
   * Delay between retries in ms
   */
  retryDelay?: number

  /**
   * Timeout for the operation in ms
   */
  timeoutMs?: number

  /**
   * Whether to cache successful results
   */
  cache?: boolean

  /**
   * Cache expiration time in ms
   */
  cacheExpirationMs?: number

  /**
   * Whether to log debug information
   */
  debug?: boolean
}

/**
 * An asynchronous operation that can be executed and observed
 */
export interface AsyncOperation<T, P = void, E = Error> {
  /**
   * Execute the operation with optional parameters
   */
  execute: (params: P) => Observable<T>

  /**
   * Observable of the operation result
   */
  result$: Observable<AsyncResult<T, E>>

  /**
   * Observable of just the data from successful operations
   */
  data$: Observable<T>

  /**
   * Current result state
   */
  getResult: () => AsyncResult<T, E>

  /**
   * Reset the operation state
   */
  reset: () => void

  /**
   * Cancel any ongoing operation
   */
  cancel: () => void
}

/**
 * Create an initial AsyncResult in IDLE state
 */
export function createInitialAsyncResult<T, E = Error>(): AsyncResult<T, E> {
  return {
    status: AsyncStatus.IDLE,
    data: null,
    error: null,
    startTime: null,
    endTime: null,
    isPending: false,
    isSuccess: false,
    isError: false
  }
}

/**
 * Create a pending AsyncResult
 */
export function createPendingAsyncResult<T, E = Error>(): AsyncResult<T, E> {
  return {
    status: AsyncStatus.PENDING,
    data: null,
    error: null,
    startTime: Date.now(),
    endTime: null,
    isPending: true,
    isSuccess: false,
    isError: false
  }
}

/**
 * Create a success AsyncResult
 */
export function createSuccessAsyncResult<T, E = Error>(
  data: T,
  startTime: number
): AsyncResult<T, E> {
  return {
    status: AsyncStatus.SUCCESS,
    data,
    error: null,
    startTime,
    endTime: Date.now(),
    isPending: false,
    isSuccess: true,
    isError: false
  }
}

/**
 * Create an error AsyncResult
 */
export function createErrorAsyncResult<T, E = Error>(
  error: E,
  startTime: number
): AsyncResult<T, E> {
  return {
    status: AsyncStatus.ERROR,
    data: null,
    error,
    startTime,
    endTime: Date.now(),
    isPending: false,
    isSuccess: false,
    isError: true
  }
}

/**
 * Create an asynchronous operation that can be executed and observed
 *
 * @param asyncFn Function that performs the async operation
 * @param options Options for the operation
 * @returns AsyncOperation object
 */
export function createAsyncOperation<T, P = void, E = Error>(
  asyncFn: (params: P) => Promise<T> | Observable<T>,
  options: AsyncOperationOptions<T, E> = {}
): AsyncOperation<T, P, E> {
  const {
    name = 'AsyncOperation',
    onSuccess,
    onError,
    retry: shouldRetry = false,
    retryCount = 3,
    retryDelay = 1000,
    timeoutMs,
    cache: shouldCache = false,
    cacheExpirationMs = 60000,
    debug = false
  } = options

  // Subject for executing the operation
  const execute$ = new Subject<P>()

  // Store the last parameters
  let lastParams: P | null = null

  // Subject for cancellation
  const cancel$ = new Subject<void>()

  // Subject for the operation result
  const resultSubject = new BehaviorSubject<AsyncResult<T, E>>(createInitialAsyncResult<T, E>())

  // Cache for results
  const resultCache = new Map<string, { data: T; expiresAt: number }>()

  // Function to log debug info
  const log = debug
    ? (message: string, ...data: any[]) => console.log(`[${name}] ${message}`, ...data)
    : () => {}

  // Convert parameters to cache key
  const getCacheKey = (params: P): string => {
    if (params === null || params === undefined) {
      return 'null'
    }
    if (typeof params === 'string' || typeof params === 'number' || typeof params === 'boolean') {
      return String(params)
    }
    try {
      return JSON.stringify(params)
    } catch {
      return 'unstringifiable'
    }
  }

  // Check if a cached result is available and valid
  const getCachedResult = (params: P): T | null => {
    if (!shouldCache) {
      return null
    }

    const key = getCacheKey(params)
    const cached = resultCache.get(key)

    if (!cached) {
      return null
    }

    // Check if cache has expired
    if (cached.expiresAt < Date.now()) {
      resultCache.delete(key)
      return null
    }

    log('Using cached result for params:', params)
    return cached.data
  }

  // Store result in cache
  const cacheResult = (params: P, data: T): void => {
    if (!shouldCache) {
      return
    }

    const key = getCacheKey(params)
    const expiresAt = Date.now() + cacheExpirationMs

    resultCache.set(key, { data, expiresAt })
    log('Cached result for params:', params, 'expires at:', new Date(expiresAt).toISOString())
  }

  // Setup the operation pipeline
  execute$
    .pipe(
      tap((params) => {
        log('Executing with params:', params)
        lastParams = params

        // First check the cache
        const cachedResult = getCachedResult(params)
        if (cachedResult !== null) {
          const result = createSuccessAsyncResult<T, E>(cachedResult, Date.now())
          resultSubject.next(result)

          if (onSuccess) {
            onSuccess(cachedResult)
          }

          return
        }

        // Otherwise start the async operation
        resultSubject.next(createPendingAsyncResult<T, E>())
      }),

      // Save the start time for non-cached executions
      map((params) => ({ params, startTime: Date.now() })),

      // Switch to the actual async operation
      switchMap(({ params, startTime }) => {
        // Try to get from cache first (might have been updated since the tap)
        const cachedResult = getCachedResult(params)
        if (cachedResult !== null) {
          return of(cachedResult)
        }

        // Convert to observable
        const source$ = defer(() => {
          try {
            const result = asyncFn(params)
            return result instanceof Promise ? from(result) : result
          } catch (error) {
            return throwError(() => error)
          }
        })

        // Apply timeout if specified
        const withTimeout$ = timeoutMs ? source$.pipe(timeout(timeoutMs)) : source$

        // Apply retry if specified
        const withRetry$ = shouldRetry
          ? withTimeout$.pipe(
              retryWhen((errors) =>
                errors.pipe(
                  mergeMap((error, index) => {
                    if (index >= retryCount) {
                      return throwError(() => error)
                    }

                    log(`Retrying (${index + 1}/${retryCount}) after error:`, error)
                    return timer(retryDelay * Math.pow(2, index))
                  })
                )
              )
            )
          : withTimeout$

        // Return the operation with cancellation and metadata
        return withRetry$.pipe(
          takeUntil(cancel$),
          map((result) => ({ result, startTime })),
          catchError((error) => of({ error, startTime }))
        )
      })
    )
    .subscribe({
      next: (result: any) => {
        if ('error' in result) {
          // Handle error result
          const errorResult = createErrorAsyncResult<T, E>(result.error as E, result.startTime)
          resultSubject.next(errorResult)

          if (onError) {
            onError(result.error as E)
          }

          log('Operation failed:', result.error)
        } else {
          // Handle success result
          const successResult = createSuccessAsyncResult<T, E>(result.result, result.startTime)
          resultSubject.next(successResult)

          // Cache the result if enabled
          if (shouldCache && lastParams !== null) {
            cacheResult(lastParams, result.result)
          }

          if (onSuccess) {
            onSuccess(result.result)
          }

          log('Operation succeeded:', result.result)
        }
      }
    })

  // Create observables for the operation result and data
  const result$ = resultSubject.asObservable().pipe(shareReplay(1))

  const data$ = result$.pipe(
    filter((result) => result.status === AsyncStatus.SUCCESS),
    map((result) => result.data!),
    distinctUntilChanged()
  )

  // Return the public API
  return {
    execute: (params: P) => {
      execute$.next(params)

      // Return a new observable that completes when the operation completes
      return result$.pipe(
        filter(
          (result) => result.status === AsyncStatus.SUCCESS || result.status === AsyncStatus.ERROR
        ),
        take(1),
        map((result) => {
          if (result.status === AsyncStatus.ERROR) {
            throw result.error as E
          }
          return result.data as T
        })
      )
    },

    result$,
    data$,

    getResult: () => resultSubject.getValue(),

    reset: () => {
      resultSubject.next(createInitialAsyncResult<T, E>())
      log('Operation reset')
    },

    cancel: () => {
      if (resultSubject.getValue().status === AsyncStatus.PENDING) {
        cancel$.next()
        resultSubject.next(createInitialAsyncResult<T, E>())
        log('Operation cancelled')
      }
    }
  }
}

/**
 * Chain multiple async operations together
 *
 * @param operations Array of async operations to chain
 * @param options Options for the combined operation
 * @returns Combined async operation
 */
export function chainAsyncOperations<T, P = void, E = Error>(
  operations: Array<(result: any) => AsyncOperation<any, any, E>>,
  options: AsyncOperationOptions<T, E> = {}
): AsyncOperation<T, P, E> {
  const { name = 'ChainedOperation', debug = false } = options

  // Create a new operation that chains the results
  return createAsyncOperation<T, P, E>(
    (params: P) => {
      // Start with an observable of the params
      return of(params).pipe(
        // Chain each operation
        concatMap((result, index) => {
          const operation = operations[index](result)
          return operation.execute(result)
        })
      )
    },
    { ...options, name, debug }
  )
}

/**
 * Execute multiple async operations in parallel
 *
 * @param operations Object mapping result keys to operations
 * @param options Options for the combined operation
 * @returns Combined async operation with results mapped to keys
 */
export function parallelAsyncOperations<T extends Record<string, any>, P = void, E = Error>(
  operations: {
    [K in keyof T]: AsyncOperation<T[K], P, E>
  },
  options: AsyncOperationOptions<T, E> = {}
): AsyncOperation<T, P, E> {
  const { name = 'ParallelOperation', debug = false } = options

  // Create a new operation that runs all operations in parallel
  return createAsyncOperation<T, P, E>(
    (params: P) => {
      // Transform operations object to observables
      const observables: Record<string, Observable<any>> = {}

      for (const key in operations) {
        observables[key] = operations[key].execute(params)
      }

      // Get all keys
      const keys = Object.keys(operations)

      // Create array of observables
      const sources = keys.map((key) =>
        observables[key].pipe(
          // Map to a tuple with the key
          map((result) => ({ key, result }))
        )
      )

      // Combine all results into a single object
      return merge(...sources).pipe(
        // Aggregate results into an object
        scan(
          (acc, { key, result }) => {
            return { ...acc, [key]: result }
          },
          {} as Record<string, any>
        ),
        // Only emit when we have all results
        filter((result: Record<string, any>) => {
          const resultKeys = Object.keys(result)
          return keys.every((key) => resultKeys.includes(key))
        }),
        // Take only the first complete result
        take(1),
        // Convert to the expected type
        map((result) => result as T)
      )
    },
    { ...options, name, debug }
  )
}

/**
 * Create an operator that handles loading states for async operations
 *
 * @param setLoading Function to set the loading state
 * @returns RxJS operator
 */
export function withLoadingState<T>(
  setLoading: (isLoading: boolean) => void
): OperatorFunction<T, T> {
  return (source: Observable<T>) => {
    return source.pipe(
      tap({
        subscribe: () => setLoading(true),
        finalize: () => setLoading(false)
      })
    )
  }
}

/**
 * Create an operator that shows a notification on success/error
 *
 * @param onSuccess Function called on successful completion
 * @param onError Function called on error
 * @returns RxJS operator
 */
export function withNotification<T>(
  onSuccess?: (result: T) => void,
  onError?: (error: any) => void
): OperatorFunction<T, T> {
  return (source: Observable<T>) => {
    return source.pipe(
      tap({
        next: (value) => {
          if (onSuccess) {
            onSuccess(value)
          }
        },
        error: (err) => {
          if (onError) {
            onError(err)
          }
        }
      })
    )
  }
}
