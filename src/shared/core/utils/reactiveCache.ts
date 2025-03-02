import { Observable, Subject, of, throwError, from, timer, ReplaySubject } from 'rxjs'
import {
  map,
  mergeMap,
  shareReplay,
  tap,
  take,
  filter,
  takeUntil,
  distinctUntilChanged
} from 'rxjs/operators'

/**
 * Cache entry with value and expiration time
 */
interface CacheEntry<T> {
  value: T
  expiresAt: number | null
}

/**
 * Options for reactive cache
 */
export interface ReactiveCacheOptions {
  /**
   * Default expiration time in milliseconds
   * If null, cache entries never expire
   */
  defaultExpirationMs?: number | null

  /**
   * Maximum number of entries to keep in cache
   * If limit is reached, least recently used entries are removed
   */
  maxEntries?: number

  /**
   * Whether to log cache operations
   */
  debug?: boolean
}

/**
 * Reactive cache that can be used to store and retrieve values
 */
export class ReactiveCache {
  /**
   * Map of cache keys to subjects
   */
  private cache = new Map<string, ReplaySubject<CacheEntry<any>>>()

  /**
   * Queue of keys in order of access (most recent at the end)
   */
  private accessQueue: string[] = []

  /**
   * Subject that emits when cache is invalidated
   */
  private invalidateSubject = new Subject<string | null>()

  /**
   * Subject that emits when an entry is removed
   */
  private removedSubject = new Subject<string>()

  /**
   * Observable of invalidation events
   */
  public readonly invalidate$ = this.invalidateSubject.asObservable()

  /**
   * Observable of removed keys
   */
  public readonly removed$ = this.removedSubject.asObservable()

  /**
   * Default options
   */
  private options: Required<ReactiveCacheOptions> = {
    defaultExpirationMs: 60000, // 1 minute
    maxEntries: 1000,
    debug: false
  }

  /**
   * Create a new reactive cache
   */
  constructor(options: ReactiveCacheOptions = {}) {
    this.options = { ...this.options, ...options }

    // Set up automatic expiration
    if (this.options.defaultExpirationMs !== null) {
      timer(0, 5000)
        .pipe(takeUntil(this.invalidateSubject.pipe(filter((key) => key === null))))
        .subscribe(() => this.removeExpiredEntries())
    }
  }

  /**
   * Get or create a cached observable
   *
   * @param key Cache key
   * @param factory Factory function to create the value if not cached
   * @param expirationMs Expiration time in milliseconds (overrides default)
   * @returns Observable of the cached value
   */
  public get<T>(
    key: string,
    factory: () => T | Promise<T> | Observable<T>,
    expirationMs?: number | null
  ): Observable<T> {
    this.updateAccessQueue(key)

    // Check if we have a cached subject for this key
    if (!this.cache.has(key)) {
      this.createCacheEntry(key, factory, expirationMs)
    } else if (this.options.debug) {
      console.log(`[ReactiveCache] Cache hit for key "${key}"`)
    }

    return this.getCacheObservable<T>(key)
  }

  /**
   * Set a value in the cache
   *
   * @param key Cache key
   * @param value Value to cache
   * @param expirationMs Expiration time in milliseconds (overrides default)
   */
  public set<T>(key: string, value: T, expirationMs?: number | null): void {
    this.updateAccessQueue(key)

    const expiration = this.calculateExpiration(expirationMs)

    if (!this.cache.has(key)) {
      // Create new subject
      const subject = new ReplaySubject<CacheEntry<T>>(1)
      this.cache.set(key, subject)
    }

    // Get existing subject and update value
    const subject = this.cache.get(key) as ReplaySubject<CacheEntry<T>>
    subject.next({ value, expiresAt: expiration })

    if (this.options.debug) {
      console.log(
        `[ReactiveCache] Set value for key "${key}", expires at ${expiration ? new Date(expiration).toISOString() : 'never'}`
      )
    }
  }

  /**
   * Check if a key exists in the cache and is not expired
   *
   * @param key Cache key
   * @returns True if key exists and is not expired
   */
  public has(key: string): Observable<boolean> {
    if (!this.cache.has(key)) {
      return of(false)
    }

    return this.cache.get(key)!.pipe(
      take(1),
      map((entry) => {
        if (entry.expiresAt === null) {
          return true
        }
        return entry.expiresAt > Date.now()
      })
    )
  }

  /**
   * Remove a key from the cache
   *
   * @param key Cache key
   */
  public remove(key: string): void {
    if (this.cache.has(key)) {
      const subject = this.cache.get(key)!
      this.cache.delete(key)
      subject.complete()
      this.removeFromAccessQueue(key)
      this.removedSubject.next(key)

      if (this.options.debug) {
        console.log(`[ReactiveCache] Removed key "${key}"`)
      }
    }
  }

  /**
   * Invalidate all cache entries or entries matching a key pattern
   *
   * @param keyPattern Optional key pattern to match (exact match or regex)
   */
  public invalidate(keyPattern?: string | RegExp): void {
    if (!keyPattern) {
      // Clear entire cache
      this.cache.forEach((subject) => subject.complete())
      this.cache.clear()
      this.accessQueue = []
      this.invalidateSubject.next(null)

      if (this.options.debug) {
        console.log(`[ReactiveCache] Invalidated entire cache`)
      }
      return
    }

    // Invalidate by pattern
    const isRegExp = keyPattern instanceof RegExp
    const keysToRemove: string[] = []

    this.cache.forEach((_, key) => {
      if (isRegExp) {
        if ((keyPattern as RegExp).test(key)) {
          keysToRemove.push(key)
        }
      } else if (key === keyPattern) {
        keysToRemove.push(key)
      }
    })

    keysToRemove.forEach((key) => this.remove(key))

    if (keysToRemove.length > 0) {
      this.invalidateSubject.next(keyPattern.toString())

      if (this.options.debug) {
        console.log(
          `[ReactiveCache] Invalidated ${keysToRemove.length} entries matching "${keyPattern}"`
        )
      }
    }
  }

  /**
   * Create a cache entry
   */
  private createCacheEntry<T>(
    key: string,
    factory: () => T | Promise<T> | Observable<T>,
    expirationMs?: number | null
  ): void {
    // Ensure we have room in the cache
    this.enforceMaxEntries()

    // Calculate expiration time
    const expiration = this.calculateExpiration(expirationMs)

    // Create subject
    const subject = new ReplaySubject<CacheEntry<T>>(1)
    this.cache.set(key, subject)

    // Convert factory result to observable
    let source$: Observable<T>
    try {
      const result = factory()
      if (result instanceof Observable) {
        source$ = result
      } else if (result instanceof Promise) {
        source$ = from(result)
      } else {
        source$ = of(result)
      }
    } catch (error) {
      source$ = throwError(() => error)
    }

    // Subscribe to source and update cache
    source$
      .pipe(
        take(1),
        tap({
          next: () => {
            if (this.options.debug) {
              console.log(
                `[ReactiveCache] Created entry for key "${key}", expires at ${expiration ? new Date(expiration).toISOString() : 'never'}`
              )
            }
          },
          error: (error) => {
            console.error(`[ReactiveCache] Error creating entry for key "${key}":`, error)
          }
        })
      )
      .subscribe({
        next: (value) => subject.next({ value, expiresAt: expiration }),
        error: (error) => {
          subject.error(error)
          this.remove(key)
        }
      })
  }

  /**
   * Get observable for a cache entry
   */
  private getCacheObservable<T>(key: string): Observable<T> {
    return this.cache.get(key)!.pipe(
      mergeMap((entry) => {
        // Check if entry is expired
        if (entry.expiresAt !== null && entry.expiresAt <= Date.now()) {
          this.remove(key)
          return throwError(() => new Error(`Cache entry for key "${key}" has expired`))
        }
        return of(entry.value)
      }),
      distinctUntilChanged(),
      shareReplay(1)
    )
  }

  /**
   * Remove expired entries from the cache
   */
  private removeExpiredEntries(): void {
    const now = Date.now()
    const keysToRemove: string[] = []

    this.cache.forEach((subject, key) => {
      // Take only the latest value to check expiration
      subject.pipe(take(1)).subscribe((entry) => {
        if (entry.expiresAt !== null && entry.expiresAt <= now) {
          keysToRemove.push(key)
        }
      })
    })

    keysToRemove.forEach((key) => this.remove(key))

    if (keysToRemove.length > 0 && this.options.debug) {
      console.log(`[ReactiveCache] Removed ${keysToRemove.length} expired entries`)
    }
  }

  /**
   * Calculate expiration time
   */
  private calculateExpiration(expirationMs?: number | null): number | null {
    // If explicitly set to null, never expire
    if (expirationMs === null) {
      return null
    }

    // Use provided expiration time or default
    const expiration = expirationMs ?? this.options.defaultExpirationMs

    // If default is null, never expire
    if (expiration === null) {
      return null
    }

    return Date.now() + expiration
  }

  /**
   * Update the access queue for LRU tracking
   */
  private updateAccessQueue(key: string): void {
    // Remove key if it already exists
    this.removeFromAccessQueue(key)

    // Add to end of queue (most recently used)
    this.accessQueue.push(key)
  }

  /**
   * Remove a key from the access queue
   */
  private removeFromAccessQueue(key: string): void {
    const index = this.accessQueue.indexOf(key)
    if (index !== -1) {
      this.accessQueue.splice(index, 1)
    }
  }

  /**
   * Enforce maximum entries limit by removing least recently used entries
   */
  private enforceMaxEntries(): void {
    if (this.options.maxEntries && this.cache.size >= this.options.maxEntries) {
      // Remove oldest entries until we're under the limit
      while (this.cache.size >= this.options.maxEntries && this.accessQueue.length > 0) {
        const oldestKey = this.accessQueue.shift()!
        this.remove(oldestKey)

        if (this.options.debug) {
          console.log(`[ReactiveCache] Removed oldest entry "${oldestKey}" to stay under limit`)
        }
      }
    }
  }
}

/**
 * Create a reactive cache with the given options
 */
export function createReactiveCache(options: ReactiveCacheOptions = {}): ReactiveCache {
  return new ReactiveCache(options)
}

/**
 * Global default cache instance
 */
export const globalCache = createReactiveCache({
  defaultExpirationMs: 5 * 60 * 1000, // 5 minutes
  maxEntries: 1000
})
