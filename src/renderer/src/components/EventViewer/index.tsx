import { useState, useEffect, useMemo } from 'react'
import { EventCategory } from '@s/types/events'
import { EventFilterCriteria, createEventFilter } from '@s/utils/filters/event-filter'
import { useFilteredEvents, useEventPublisher } from '@r/hooks/useEventStream'
import EventFilterBar from './EventFilterBar'
import EventGroup from './EventGroup'
import EventTimeline from './EventTimeline'
import { groupEvents, extractSources, extractEventTypes, orderGroups, GroupBy } from './eventUtils'

interface EventViewerProps {
  title?: string
  initialCategory?: EventCategory
  initialType?: string
  initialGroupBy?: GroupBy
  showTimeline?: boolean
  showTestControls?: boolean
  maxEventsPerGroup?: number
}

/**
 * Comprehensive event viewer component with filtering, grouping, and timeline visualization
 */
export function EventViewer({
  title = 'Event Viewer',
  initialCategory,
  initialType,
  initialGroupBy = GroupBy.CATEGORY,
  showTimeline = true,
  showTestControls = false,
  maxEventsPerGroup = 100
}: EventViewerProps) {
  // Filter state
  const [filterCriteria, setFilterCriteria] = useState<EventFilterCriteria>({})
  const [timeRangeMinutes, setTimeRangeMinutes] = useState(60)
  const [groupBy, setGroupBy] = useState<GroupBy>(initialGroupBy)
  
  // Initialize filter criteria if initial values provided
  useEffect(() => {
    const criteria: EventFilterCriteria = {}
    
    if (initialCategory) {
      criteria.category = initialCategory
    }
    
    if (initialType) {
      criteria.type = initialType
    }
    
    setFilterCriteria(criteria)
  }, [initialCategory, initialType])
  
  // Apply time range filter
  useEffect(() => {
    if (timeRangeMinutes > 0) {
      const since = Date.now() - (timeRangeMinutes * 60 * 1000)
      setFilterCriteria(prev => ({
        ...prev,
        since
      }))
    }
  }, [timeRangeMinutes])
  
  // Get filtered events
  const filteredEvents = useFilteredEvents(filterCriteria)
  
  // Extract unique sources and types for filter options
  const sources = useMemo(() => extractSources(filteredEvents), [filteredEvents])
  const eventTypes = useMemo(() => extractEventTypes(filteredEvents), [filteredEvents])
  
  // Group events
  const groupedEvents = useMemo(() => {
    return orderGroups(groupEvents(filteredEvents, groupBy), groupBy)
  }, [filteredEvents, groupBy])
  
  // Test event publisher
  const publishTestEvent = useEventPublisher(
    EventCategory.TEST,
    'event-viewer-test',
    'event-viewer'
  )
  
  // Generate test event
  const handleGenerateTestEvent = () => {
    publishTestEvent({
      message: 'Test event from Event Viewer',
      timestamp: Date.now(),
      value: Math.random()
    })
  }
  
  // Handle grouping change
  const handleGroupingChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setGroupBy(e.target.value as GroupBy)
  }
  
  // Handle filter changes
  const handleFilterChange = (criteria: EventFilterCriteria) => {
    // Preserve time range filter
    if (timeRangeMinutes > 0) {
      const since = Date.now() - (timeRangeMinutes * 60 * 1000)
      setFilterCriteria({
        ...criteria,
        since
      })
    } else {
      setFilterCriteria(criteria)
    }
  }
  
  return (
    <div className="p-4 max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-2xl font-bold">{title}</h2>
        
        <div className="flex items-center gap-3">
          <div>
            <label className="text-sm mr-2">Group by:</label>
            <select
              value={groupBy}
              onChange={handleGroupingChange}
              className="p-1.5 text-sm border rounded"
            >
              <option value={GroupBy.CATEGORY}>Category</option>
              <option value={GroupBy.TYPE}>Type</option>
              <option value={GroupBy.SOURCE}>Source</option>
              <option value={GroupBy.NONE}>No Grouping</option>
            </select>
          </div>
          
          {showTestControls && (
            <button
              onClick={handleGenerateTestEvent}
              className="p-1.5 text-sm bg-blue-500 hover:bg-blue-600 text-white rounded"
            >
              Generate Test Event
            </button>
          )}
        </div>
      </div>
      
      {/* Filter bar */}
      <EventFilterBar
        onFilterChange={handleFilterChange}
        onTimeRangeChange={setTimeRangeMinutes}
        categories={Object.values(EventCategory)}
        sources={sources}
        types={eventTypes}
      />
      
      {/* Timeline visualization */}
      {showTimeline && (
        <EventTimeline
          events={filteredEvents}
          timeRange={timeRangeMinutes}
        />
      )}
      
      {/* Event groups */}
      <div className="space-y-4">
        {groupedEvents.map(([groupName, events]) => (
          <EventGroup
            key={groupName}
            title={groupName}
            events={events.slice(0, maxEventsPerGroup)}
            emptyMessage={`No ${groupName.toLowerCase()} events in the selected time range`}
          />
        ))}
        
        {groupedEvents.length === 0 && (
          <div className="bg-white p-8 text-center text-gray-500 rounded-md shadow-sm border">
            No events match the current filters
          </div>
        )}
      </div>
    </div>
  )
}

export default EventViewer

// Also export the subcomponents
export { EventFilterBar, EventGroup, EventTimeline }
export * from './eventUtils'