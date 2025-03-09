import { EventCategory } from '@s/types/events'

interface BaseEvent<T = unknown> {
  id: string
  timestamp: number
  category?: EventCategory
  type?: string
  source?: {
    id: string
    type: string
  }
  payload: T
}

export enum GroupBy {
  CATEGORY = 'category',
  TYPE = 'type',
  SOURCE = 'source',
  NONE = 'none'
}

interface GroupedEvents {
  [key: string]: BaseEvent[]
}

/**
 * Groups events by the specified property
 */
export function groupEvents(
  events: BaseEvent[],
  groupBy: GroupBy = GroupBy.CATEGORY
): GroupedEvents {
  if (groupBy === GroupBy.NONE) {
    return { 'All Events': events }
  }

  return events.reduce((groups: GroupedEvents, event) => {
    let key = 'Other'

    if (groupBy === GroupBy.CATEGORY && event.category) {
      key = event.category
    } else if (groupBy === GroupBy.TYPE && event.type) {
      key = event.type
    } else if (groupBy === GroupBy.SOURCE && event.source) {
      key = `${event.source.id} (${event.source.type})`
    }

    if (!groups[key]) {
      groups[key] = []
    }

    groups[key].push(event)
    return groups
  }, {})
}

/**
 * Extracts unique sources from a list of events
 */
export function extractSources(events: BaseEvent[]): Array<{ id: string; type: string }> {
  const sourceMap = new Map<string, { id: string; type: string }>()

  events.forEach((event) => {
    if (event.source && event.source.id && event.source.type) {
      const key = `${event.source.id}|${event.source.type}`
      sourceMap.set(key, event.source)
    }
  })

  return Array.from(sourceMap.values())
}

/**
 * Extracts unique event types from a list of events
 */
export function extractEventTypes(events: BaseEvent[]): string[] {
  const types = new Set<string>()

  events.forEach((event) => {
    if (event.type) {
      types.add(event.type)
    }
  })

  return Array.from(types)
}

/**
 * Orders groups by a preferred order or alphabetically
 */
export function orderGroups(groups: GroupedEvents, groupBy: GroupBy): [string, BaseEvent[]][] {
  const entries = Object.entries(groups)

  if (groupBy === GroupBy.CATEGORY) {
    // Define preferred order for categories
    const categoryOrder: { [key: string]: number } = {
      [EventCategory.ERROR]: 1,
      [EventCategory.SYSTEM]: 2,
      [EventCategory.ADAPTER]: 3,
      [EventCategory.AUTH]: 4,
      [EventCategory.TEST]: 5,
      Other: 99
    }

    return entries.sort((a, b) => {
      const orderA = categoryOrder[a[0]] || 50
      const orderB = categoryOrder[b[0]] || 50
      return orderA - orderB
    })
  }

  // Default to sort by group name
  return entries.sort((a, b) => a[0].localeCompare(b[0]))
}
