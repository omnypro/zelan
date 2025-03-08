import { useState } from 'react'
import { EventCategory } from '@s/types/events'
import EventCard from './EventCard'

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

interface EventGroupProps {
  title: string
  events: Array<BaseEvent>
  collapsible?: boolean
  initialCollapsed?: boolean
  maxHeight?: string
  emptyMessage?: string
}

/**
 * Displays a group of events with an optional title and collapsible content
 */
export function EventGroup({ 
  title, 
  events,
  collapsible = true,
  initialCollapsed = false,
  maxHeight = '400px',
  emptyMessage = 'No events in this group'
}: EventGroupProps) {
  const [collapsed, setCollapsed] = useState(initialCollapsed)
  
  const toggleCollapse = () => {
    if (collapsible) {
      setCollapsed(!collapsed)
    }
  }
  
  return (
    <div className="bg-white rounded-md shadow-sm border overflow-hidden mb-4">
      {/* Group Header */}
      <div 
        className={`p-3 border-b bg-gray-50 flex justify-between items-center ${collapsible ? 'cursor-pointer hover:bg-gray-100' : ''}`}
        onClick={toggleCollapse}
      >
        <h3 className="text-md font-semibold">
          {title} 
          <span className="text-sm font-normal text-gray-500 ml-2">
            ({events.length})
          </span>
        </h3>
        
        {collapsible && (
          <button className="text-gray-500 hover:text-gray-700">
            {collapsed ? (
              <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clipRule="evenodd" />
              </svg>
            ) : (
              <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M14.707 12.707a1 1 0 01-1.414 0L10 9.414l-3.293 3.293a1 1 0 01-1.414-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 010 1.414z" clipRule="evenodd" />
              </svg>
            )}
          </button>
        )}
      </div>
      
      {/* Group Content */}
      {!collapsed && (
        <div 
          className="divide-y overflow-y-auto" 
          style={{ maxHeight }}
        >
          {events.length === 0 ? (
            <div className="p-4 text-sm text-gray-500 text-center">
              {emptyMessage}
            </div>
          ) : (
            events.map((event) => (
              <EventCard
                key={event.id}
                id={event.id}
                timestamp={event.timestamp}
                category={event.category}
                type={event.type}
                source={event.source}
                payload={event.payload}
              />
            ))
          )}
        </div>
      )}
    </div>
  )
}

export default EventGroup