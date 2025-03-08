import { useState } from 'react'
import { EventCategory } from '@s/types/events'
import { useEventPublisher } from '@r/hooks/useEventStream'
import EventViewer, { GroupBy } from './EventViewer'

/**
 * Demo component for the enhanced event viewer
 */
export function EnhancedEventsDemo() {
  const [eventCount, setEventCount] = useState(0)
  
  // Create publishers for different event categories
  const publishSystemEvent = useEventPublisher(
    EventCategory.SYSTEM,
    'info',
    'events-demo'
  )
  
  const publishAdapterEvent = useEventPublisher(
    EventCategory.ADAPTER,
    'status',
    'test-adapter'
  )
  
  const publishErrorEvent = useEventPublisher(
    EventCategory.ERROR,
    'fatal',
    'events-demo'
  )
  
  const publishAuthEvent = useEventPublisher(
    EventCategory.AUTH,
    'token',
    'auth-service'
  )
  
  // Generate various test events
  const generateEvents = () => {
    // System event
    publishSystemEvent({
      message: `System test event #${eventCount + 1}`,
      timestamp: Date.now()
    })
    
    // Adapter event
    const statuses = ['connected', 'disconnected', 'connecting', 'error']
    publishAdapterEvent({
      status: statuses[Math.floor(Math.random() * statuses.length)],
      message: `Adapter status changed for test #${eventCount + 1}`,
      timestamp: Date.now()
    })
    
    // Error event (occasionally)
    if (Math.random() > 0.7) {
      publishErrorEvent({
        message: `Test error occurred #${eventCount + 1}`,
        code: 'TEST_ERROR',
        timestamp: Date.now()
      })
    }
    
    // Auth event (occasionally)
    if (Math.random() > 0.8) {
      publishAuthEvent({
        action: 'refresh',
        service: 'test-service',
        success: Math.random() > 0.3,
        timestamp: Date.now()
      })
    }
    
    setEventCount(prev => prev + 1)
  }
  
  return (
    <div className="p-4">
      <div className="mb-6 max-w-6xl mx-auto flex justify-between items-center">
        <h1 className="text-3xl font-bold">Enhanced Event Viewer</h1>
        
        <div className="flex gap-3">
          <button
            onClick={generateEvents}
            className="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded"
          >
            Generate Test Events
          </button>
        </div>
      </div>
      
      <EventViewer 
        title="All Events"
        initialGroupBy={GroupBy.CATEGORY}
        showTimeline={true}
      />
    </div>
  )
}

export default EnhancedEventsDemo