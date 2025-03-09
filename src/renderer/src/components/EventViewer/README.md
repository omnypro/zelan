# Enhanced Event Viewer

A comprehensive set of components for visualizing, filtering, and organizing events in the application.

## Features

- **Advanced Filtering**: Filter events by category, type, source, and time range
- **Timeline Visualization**: See event frequency distribution over time
- **Customizable Grouping**: Group events by category, type, source, or display all together
- **Expandable Event Details**: Click on events to see full details and raw payload
- **Smart Event Formatting**: Automatic formatting based on event type and content
- **Type-Safe Implementation**: Full TypeScript support throughout

## Components

### EventViewer

The main component that orchestrates all event visualization functionality.

```tsx
import EventViewer, { GroupBy } from './components/EventViewer'

function MyComponent() {
  return (
    <EventViewer
      title="System Events"
      initialCategory={EventCategory.SYSTEM}
      initialGroupBy={GroupBy.TYPE}
      showTimeline={true}
      showTestControls={true}
      maxEventsPerGroup={50}
    />
  )
}
```

#### Props

- `title`: Title displayed at the top of the viewer
- `initialCategory`: Pre-select a specific event category
- `initialType`: Pre-select a specific event type
- `initialGroupBy`: How events should be grouped (category, type, source, or none)
- `showTimeline`: Whether to show the timeline visualization
- `showTestControls`: Whether to show test event generation controls
- `maxEventsPerGroup`: Maximum number of events to display in each group

### EventFilterBar

Filter controls for events with support for category, type, source, and time range.

```tsx
import { EventFilterBar } from './components/EventViewer'

function MyComponent() {
  const handleFilterChange = (criteria) => {
    // Apply filters
  }

  return (
    <EventFilterBar
      onFilterChange={handleFilterChange}
      onTimeRangeChange={(minutes) => console.info(`Time range: ${minutes} minutes`)}
      categories={Object.values(EventCategory)}
      sources={mySources}
      types={myEventTypes}
    />
  )
}
```

### EventGroup

Displays a collapsible group of events with a title.

```tsx
import { EventGroup } from './components/EventViewer'

function MyComponent() {
  return (
    <EventGroup
      title="System Errors"
      events={myEvents}
      collapsible={true}
      initialCollapsed={false}
      maxHeight="300px"
      emptyMessage="No system errors found"
    />
  )
}
```

### EventTimeline

Visualizes event frequency over time with category-based coloring.

```tsx
import { EventTimeline } from './components/EventViewer'

function MyComponent() {
  return (
    <EventTimeline
      events={myEvents}
      timeRange={60} // minutes
      height={100}
      onTimeClick={(timestamp) => console.info(`Clicked on ${new Date(timestamp)}`)}
    />
  )
}
```

### EventCard

Displays a single event with expandable details.

```tsx
import EventCard from './components/EventViewer/EventCard'

function MyComponent() {
  return (
    <EventCard
      id="event-123"
      timestamp={Date.now()}
      category={EventCategory.SYSTEM}
      type="info"
      source={{ id: 'system', type: 'core' }}
      payload={{ message: 'System started' }}
      expanded={false}
      onClick={() => console.info('Event clicked')}
    />
  )
}
```

## Utility Functions

The EventViewer also comes with several utility functions:

- `groupEvents(events, groupBy)`: Groups events by category, type, source, or none
- `extractSources(events)`: Extracts unique sources from a list of events
- `extractEventTypes(events)`: Extracts unique event types from a list of events
- `orderGroups(groups, groupBy)`: Orders groups in a logical way based on grouping type

## Usage Examples

### Basic Event Viewer

```tsx
import EventViewer from './components/EventViewer'

function EventsPage() {
  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold mb-4">Event Monitor</h1>
      <EventViewer />
    </div>
  )
}
```

### Category-Specific Viewer

```tsx
import EventViewer from './components/EventViewer'
import { EventCategory } from '@s/types/events'

function ErrorsPage() {
  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold mb-4">Error Log</h1>
      <EventViewer title="Application Errors" initialCategory={EventCategory.ERROR} showTestControls={false} />
    </div>
  )
}
```

### Custom Grouped Events

```tsx
import { useState, useMemo } from 'react'
import { EventGroup, groupEvents, GroupBy } from './components/EventViewer'
import { useFilteredEvents } from '@r/hooks/useEventStream'

function CustomEventsView() {
  const [criteria, setCriteria] = useState({ since: Date.now() - 3600000 }) // Last hour
  const events = useFilteredEvents(criteria)

  const groupedEvents = useMemo(() => {
    return groupEvents(events, GroupBy.TYPE)
  }, [events])

  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold mb-4">Recent Events by Type</h1>

      {Object.entries(groupedEvents).map(([type, events]) => (
        <EventGroup key={type} title={type} events={events} />
      ))}
    </div>
  )
}
```
