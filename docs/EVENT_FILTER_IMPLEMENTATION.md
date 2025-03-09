# Event Filtering Implementation

## Overview

This document summarizes the implementation of the standardized event filtering system in the Zelan application. The goal was to create a unified approach to event filtering that works consistently across both reactive (Observable-based) and imperative (Array-based) code.

## Key Components

### 1. Event Filter Utility (`src/shared/utils/filters/event-filter.ts`)

The core of the implementation is a set of utility functions for filtering events:

- `EventFilterCriteria<T>`: Interface for defining filter criteria
- `createEventFilterPredicate()`: Creates a reusable filter predicate function
- `filterEvents()`: Filters an array of events based on criteria
- `filterEventStream()`: Creates an RxJS operator for filtering Observable streams
- `EventFilterBuilder`: Builder class with fluent API for creating complex filters
- `createEventFilter()`: Factory function for creating filter builders

### 2. EventBus Updates (`src/shared/core/bus/EventBus.ts`)

The EventBus interface and implementation were updated to use the new filtering system:

- Added `getFilteredEvents$<T>()` method to support generalized filtering
- Updated existing filter methods to use the new system internally
- Improved type safety with proper generic type handling

### 3. EventCache Updates (`src/main/services/events/EventCache.ts`)

The EventCache was updated to use the standardized filtering:

- Added `EventCacheOptions` interface extending `EventFilterCriteria`
- Updated `getEvents()` to use the `filterEvents` utility
- Updated `filteredEvents$()` to use the same filtering approach

### 4. React Hooks Updates (`src/renderer/src/hooks/useEventStream.ts`)

New React hooks were added to leverage the filtering system:

- `useFilteredEventStream()`: Creates a filtered event stream with custom criteria
- `useFilteredEvents()`: Subscribes to events with custom filter criteria
- `useFilteredEventPayload()`: Gets the latest event payload with custom filtering

### 5. Demo Component (`src/renderer/src/components/EventFilterDemo.tsx`)

A demonstration component was created to showcase the filtering capabilities:

- UI for configuring filter criteria
- Real-time filtering of events
- Display of filtered events

## Key Features

1. **Unified Filtering API**: Consistent approach for both arrays and Observables
2. **Type Safety**: Full TypeScript support with generic types
3. **Fluent Builder API**: Easy-to-use builder pattern for complex filters
4. **Composable Filters**: Combine multiple filter criteria
5. **Optimized Performance**: Efficient filtering implementation
6. **Extended Criteria**: Support for filtering by source, timestamp, and custom predicates

## Usage Examples

### Basic Filtering

```typescript
// Array filtering
const filteredEvents = filterEvents(events, {
  category: EventCategory.ADAPTER,
  type: 'connection'
})

// Observable filtering
const filteredStream$ = sourceStream$.pipe(
  filterEventStream({
    category: EventCategory.ADAPTER,
    sourceType: 'twitch'
  })
)
```

### Fluent Builder API

```typescript
const filter = createEventFilter()
  .byCategory(EventCategory.ADAPTER)
  .byType('connection')
  .bySourceId('twitch-1')
  .since(Date.now() - 3600000) // Last hour
  .build()

// Apply to arrays
const filtered = filterEvents(events, filter)

// Apply to streams
const filteredStream$ = stream$.pipe(filterEventStream(filter))
```

### React Hooks

```typescript
// Get filtered events with custom criteria
const events = useFilteredEvents({
  category: EventCategory.ADAPTER,
  sourceType: 'twitch'
})

// Get the latest payload with custom filtering
const { payload, loading, error } = useFilteredEventPayload({
  category: EventCategory.ADAPTER,
  type: 'status'
})
```

## Benefits

1. **Code Reusability**: Eliminates duplicate filtering logic
2. **Consistency**: Ensures filtering works the same way everywhere
3. **Maintainability**: Centralizes filtering logic in one place
4. **Extensibility**: Easy to add new filtering capabilities
5. **Performance**: Optimized filtering implementation

## Next Steps

1. **Memoization**: Add caching for frequently used filter combinations
2. **Filter Presets**: Create common filter presets for reuse
3. **Advanced Filtering**: Add more sophisticated filtering options (regex, etc.)
4. **Filter Persistence**: Allow saving and loading filter configurations
