import {
  Observable,
  Subject,
  BehaviorSubject,
  combineLatest,
  interval,
  from,
  EMPTY,
  OperatorFunction,
  MonoTypeOperatorFunction,
  animationFrameScheduler
} from 'rxjs'
import {
  map,
  filter,
  distinctUntilChanged,
  debounceTime,
  throttleTime,
  catchError,
  tap,
  scan,
  share,
  shareReplay,
  bufferTime
} from 'rxjs/operators'

/**
 * Options for creating a data stream
 */
export interface DataStreamOptions<T> {
  /**
   * Initial value for the stream
   */
  initialValue?: T

  /**
   * Buffer time in milliseconds for throttling updates
   */
  bufferTimeMs?: number

  /**
   * Debounce time in milliseconds for delaying updates
   */
  debounceTimeMs?: number

  /**
   * Name for debugging purposes
   */
  name?: string

  /**
   * Whether to log events for debugging
   */
  debug?: boolean

  /**
   * Cleanup function to run when all subscriptions are removed
   */
  cleanup?: () => void

  /**
   * Whether to share the stream with multiple subscribers
   */
  share?: boolean

  /**
   * Whether to replay the latest value to new subscribers
   */
  replay?: boolean

  /**
   * Number of values to replay (only used if replay is true)
   */
  replayCount?: number

  /**
   * Whether to use animation frame scheduling for updates
   */
  useAnimationFrame?: boolean

  /**
   * Whether to skip duplicate values using distinctUntilChanged
   */
  distinctValues?: boolean

  /**
   * Equality comparator for distinct values
   */
  compareValues?: (previous: T, current: T) => boolean
}

/**
 * Result of creating a data stream
 */
export interface DataStream<T> {
  /**
   * Observable stream of data
   */
  stream$: Observable<T>

  /**
   * Push a new value to the stream
   */
  push: (value: T) => void

  /**
   * Update the stream with a function that takes the current value
   */
  update: (updateFn: (currentValue: T) => T) => void

  /**
   * Force the stream to emit its current value again
   */
  refresh: () => void

  /**
   * Get the current value of the stream
   */
  getValue: () => T | undefined

  /**
   * Complete the stream and clean up resources
   */
  complete: () => void

  /**
   * Signal an error in the stream
   */
  error: (err: unknown) => void

  /**
   * Create a derived stream that transforms values
   */
  derive: <R>(
    transformFn: (value: T) => R,
    options?: Partial<DataStreamOptions<R>>
  ) => DataStream<R>
}

/**
 * Create a reactive data stream
 *
 * @param options Options for the stream
 * @returns Data stream object
 */
export function createDataStream<T>(options: DataStreamOptions<T> = {}): DataStream<T> {
  const {
    initialValue,
    bufferTimeMs,
    debounceTimeMs,
    name = 'DataStream',
    debug = false,
    cleanup,
    share: shouldShare = false,
    replay = true,
    replayCount = 1,
    useAnimationFrame = false,
    distinctValues = true,
    compareValues
  } = options

  // Create source subject based on whether we have an initial value
  const source =
    initialValue !== undefined ? new BehaviorSubject<T>(initialValue) : new Subject<T>()

  let currentValue: T | undefined = initialValue

  // Function to log debug info
  const log = debug
    ? (message: string, ...data: unknown[]) => {
        console.log(`[${name}] ${message}`, ...data)
      }
    : () => {}

  // Create the base pipeline
  let pipeline = source.asObservable()

  // Apply buffering if specified
  if (bufferTimeMs && bufferTimeMs > 0) {
    pipeline = pipeline.pipe(
      bufferTime(bufferTimeMs),
      filter((buffer) => buffer.length > 0),
      map((buffer) => buffer[buffer.length - 1]) // Take the most recent value
    )
  }

  // Apply debouncing if specified
  if (debounceTimeMs && debounceTimeMs > 0) {
    pipeline = pipeline.pipe(debounceTime(debounceTimeMs))
  }

  // Apply distinct filtering if specified
  if (distinctValues) {
    pipeline = pipeline.pipe(distinctUntilChanged(compareValues))
  }

  // Use animation frame scheduler if specified
  if (useAnimationFrame) {
    pipeline = pipeline.pipe(
      throttleTime(0, animationFrameScheduler, { leading: true, trailing: true })
    )
  }

  // Apply sharing/replay if specified
  if (shouldShare) {
    pipeline = replay ? pipeline.pipe(shareReplay(replayCount)) : pipeline.pipe(share())
  }

  // Track current value and log updates
  pipeline = pipeline.pipe(
    tap((value) => {
      currentValue = value
      if (debug) {
        log('Value updated:', value)
      }
    })
  )

  // Create the public API
  const dataStream: DataStream<T> = {
    stream$: pipeline,

    push: (value: T) => {
      if (debug) {
        log('Pushing value:', value)
      }
      source.next(value)
    },

    update: (updateFn: (currentValue: T) => T) => {
      if (currentValue !== undefined) {
        const newValue = updateFn(currentValue)
        if (debug) {
          log('Updating from:', currentValue, 'to:', newValue)
        }
        source.next(newValue)
      } else {
        console.warn(`[${name}] Cannot update stream: current value is undefined`)
      }
    },

    refresh: () => {
      if (currentValue !== undefined) {
        if (debug) {
          log('Refreshing with current value:', currentValue)
        }
        source.next(currentValue)
      } else {
        console.warn(`[${name}] Cannot refresh stream: current value is undefined`)
      }
    },

    getValue: () => currentValue,

    complete: () => {
      if (debug) {
        log('Completing stream')
      }
      source.complete()
      if (cleanup) {
        cleanup()
      }
    },

    error: (err: unknown) => {
      if (debug) {
        log('Stream error:', err)
      }
      source.error(err)
    },

    derive: <R>(
      transformFn: (value: T) => R,
      derivedOptions: Partial<DataStreamOptions<R>> = {}
    ): DataStream<R> => {
      // Combine options, allowing overrides
      const newOptions: DataStreamOptions<R> = {
        ...options,
        ...derivedOptions,
        name: derivedOptions.name || `${name}:derived`,
        initialValue: initialValue !== undefined ? transformFn(initialValue) : undefined,
        // Ensure compareValues is properly typed for R
        compareValues: derivedOptions.compareValues
      }

      // Create the derived stream
      const derived = createDataStream<R>(newOptions)

      // Connect the streams
      const subscription = dataStream.stream$.subscribe({
        next: (value) => {
          try {
            const transformedValue = transformFn(value)
            derived.push(transformedValue)
          } catch (err) {
            console.error(`Error in derived stream [${newOptions.name}]:`, err)
          }
        },
        error: (err) => derived.error(err),
        complete: () => derived.complete()
      })

      // Add cleanup to complete the subscription
      const originalComplete = derived.complete
      derived.complete = () => {
        subscription.unsubscribe()
        originalComplete()
      }

      return derived
    }
  }

  return dataStream
}

/**
 * Combine multiple data streams into one
 *
 * @param streams Streams to combine
 * @param combinerFn Function to combine latest values from each stream
 * @param options Options for the combined stream
 * @returns Combined data stream
 */
export function combineDataStreams<T extends any[], R>(
  streams: { [K in keyof T]: DataStream<T[K]> },
  combinerFn: (...values: T) => R,
  options: Partial<DataStreamOptions<R>> = {}
): DataStream<R> {
  // Extract observables from streams
  const observables = streams.map((stream) => stream.stream$)

  // Calculate initial value if all input streams have values
  let initialValue: R | undefined = undefined
  const allHaveValues = streams.every((stream) => stream.getValue() !== undefined)

  if (allHaveValues) {
    const initialValues = streams.map((stream) => stream.getValue()) as T
    initialValue = combinerFn(...initialValues)
  }

  // Create combined stream
  const combinedOptions: DataStreamOptions<R> = {
    ...options,
    name: options.name || 'CombinedStream',
    initialValue
  }

  const combinedStream = createDataStream<R>(combinedOptions)

  // Connect the streams
  const subscription = combineLatest(observables).subscribe({
    next: (values) => {
      const combinedValue = combinerFn(...(values as T))
      combinedStream.push(combinedValue)
    },
    error: (err) => combinedStream.error(err),
    complete: () => combinedStream.complete()
  })

  // Add cleanup to complete the subscription
  const originalComplete = combinedStream.complete
  combinedStream.complete = () => {
    subscription.unsubscribe()
    originalComplete()
  }

  return combinedStream
}

/**
 * Create an interval data stream that emits on a fixed schedule
 *
 * @param periodMs Interval period in milliseconds
 * @param valueFactory Function that produces values for each interval
 * @param options Stream options
 * @returns Data stream that emits on interval
 */
export function createIntervalStream<T>(
  periodMs: number,
  valueFactory: (count: number) => T,
  options: Partial<DataStreamOptions<T>> = {}
): DataStream<T> {
  const streamOptions: DataStreamOptions<T> = {
    ...options,
    name: options.name || 'IntervalStream'
  }

  // Create the stream without pushing initial value
  const stream = createDataStream<T>(streamOptions)

  // Create counter for the interval
  let count = 0

  // Connect interval to stream
  const subscription = interval(periodMs).subscribe({
    next: () => {
      try {
        const value = valueFactory(count++)
        stream.push(value)
      } catch (err) {
        console.error(`Error in interval stream [${streamOptions.name}]:`, err)
      }
    }
  })

  // Enhance complete to clean up the interval
  const originalComplete = stream.complete
  stream.complete = () => {
    subscription.unsubscribe()
    originalComplete()
  }

  return stream
}

/**
 * Create a polling data stream that fetches data on an interval
 *
 * @param fetchFn Function that fetches data (returns Promise or Observable)
 * @param periodMs Polling interval in milliseconds
 * @param options Stream options
 * @returns Data stream that polls for updates
 */
export function createPollingStream<T>(
  fetchFn: () => Promise<T> | Observable<T>,
  periodMs: number,
  options: Partial<DataStreamOptions<T>> = {}
): DataStream<T> {
  const streamOptions: DataStreamOptions<T> = {
    ...options,
    name: options.name || 'PollingStream',
    replay: true
  }

  // Create the stream
  const stream = createDataStream<T>(streamOptions)

  // Flag to track initial load
  let initialLoadComplete = false

  // Function to perform fetch with error handling
  const performFetch = () => {
    try {
      const result = fetchFn()

      // Convert to observable
      const source$ = result instanceof Promise ? from(result) : result

      // Subscribe to result
      source$
        .pipe(
          catchError((error) => {
            console.error(`Error in polling stream [${streamOptions.name}]:`, error)
            return EMPTY
          })
        )
        .subscribe({
          next: (value) => {
            stream.push(value)
            initialLoadComplete = true
          }
        })
    } catch (err) {
      console.error(`Error starting fetch in polling stream [${streamOptions.name}]:`, err)
    }
  }

  // Start immediate fetch
  performFetch()

  // Set up interval for polling
  const subscription = interval(periodMs).subscribe({
    next: () => {
      // Skip first interval if we're still waiting on initial load
      if (initialLoadComplete) {
        performFetch()
      }
    }
  })

  // Enhance complete to clean up the interval
  const originalComplete = stream.complete
  stream.complete = () => {
    subscription.unsubscribe()
    originalComplete()
  }

  return stream
}

/**
 * Create a filtering operator for data streams
 *
 * @param predicate Filter predicate
 * @returns Operator function
 */
export function dataFilter<T>(predicate: (value: T) => boolean): MonoTypeOperatorFunction<T> {
  return filter(predicate)
}

/**
 * Create a mapping operator for data streams
 *
 * @param mapper Mapping function
 * @returns Operator function
 */
export function dataMap<T, R>(mapper: (value: T) => R): OperatorFunction<T, R> {
  return map(mapper)
}

/**
 * Create a scan operator for accumulating values
 *
 * @param accumulator Accumulator function
 * @param seed Initial value
 * @returns Operator function
 */
export function dataAccumulate<T, R>(
  accumulator: (acc: R, value: T) => R,
  seed: R
): OperatorFunction<T, R> {
  return scan(accumulator, seed)
}
