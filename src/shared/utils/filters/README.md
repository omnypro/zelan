# Event Filtering Utilities

This module provides standardized utilities for filtering events across the application. It provides a consistent API for both reactive (Observable-based) and imperative (Array-based) filtering.

## Features

- Type-safe filtering with TypeScript generics
- Support for filtering by multiple criteria (category, type, source, timestamp, etc.)
- Fluent builder API for creating complex filters
- Compatible with both RxJS Observables and standard Arrays
- Memoization-friendly parameter structure

## Usage Examples

### Basic Filtering

```typescript
import { filterEvents, EventFilterCriteria } from '@s/utils/filters/event-filter'
import { EventCategory } from '@s/types/events'

// Define filter criteria
const criteria: EventFilterCriteria = {
  category: EventCategory.ADAPTER,
  type: 'connection',
  sourceId: 'twitch-adapter'
}

// Filter an array of events
const filteredEvents = filterEvents(allEvents, criteria)
```

### Filtering Observable Streams

```typescript
import { filterEventStream } from '@s/utils/filters/event-filter'
import { EventCategory } from '@s/types/events'

// Create a filtered Observable stream
const filteredStream$ = sourceStream$.pipe(
  filterEventStream({
    category: EventCategory.ADAPTER,
    sourceType: 'twitch'
  })
)
```

### Using the Fluent Builder API

```typescript
import { createEventFilter } from '@s/utils/filters/event-filter'
import { EventCategory } from '@s/types/events'

// Create filter with builder pattern
const filter = createEventFilter()
  .byCategory(EventCategory.ADAPTER)
  .byType('connection')
  .bySourceType('twitch')
  .since(Date.now() - 3600000) // Events from the last hour
  .where((event) => event.payload.status === 'connected')
  .build()

// Apply to arrays
const filteredArray = filterEvents(events, filter)

// Apply to Observables
const filteredStream$ = eventStream$.pipe(filterEventStream(filter))
```

### In React Components

```typescript
import { useFilteredEvents, useFilteredEventPayload } from '@r/hooks/useEventStream'
import { EventCategory } from '@s/types/events'

function ConnectionStatus() {
  // Get filtered events using the hook
  const connectionEvents = useFilteredEvents({
    category: EventCategory.ADAPTER,
    type: 'connection'
  })

  // Or use the fluent builder
  const filter = createEventFilter().byCategory(EventCategory.ADAPTER).byType('status').build()

  const status = useFilteredEventPayload(filter)

  // ...
}
```

## API Reference

### Types

- `EventFilterCriteria<T>`: Interface for filter criteria
  - `category?`: Event category
  - `type?`: Event type
  - `sourceId?`: Source ID
  - `sourceType?`: Source type
  - `since?`: Minimum timestamp
  - `predicate?`: Custom filter function

### Functions

- `createEventFilterPredicate<T>(criteria)`: Creates a filter predicate function
- `filterEvents<T>(events, criteria)`: Filters an array of events
- `filterEventStream<T>(criteria)`: Creates an RxJS operator for filtering Observables
- `createEventFilter<T>()`: Creates a new filter builder

### Builder Methods

- `byCategory(category)`: Filter by event category
- `byType(type)`: Filter by event type
- `bySourceId(sourceId)`: Filter by source ID
- `bySourceType(sourceType)`: Filter by source type
- `since(timestamp)`: Filter by minimum timestamp
- `where(predicate)`: Add a custom filter predicate
- `build()`: Get the constructed filter criteria
- `applyToArray(events)`: Apply the filter to an array
- `applyToStream(stream$)`: Apply the filter to an Observable
