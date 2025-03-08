import React, { useEffect, useState } from 'react'
import { EventCategory } from '@s/types/events'
import { useEvents, useEventPublisher } from '@r/hooks/useEventStream'
import { rendererEventBus } from '@r/services/eventBus'
import { SystemInfoEvent } from '@s/core/events'

/**
 * Component to demonstrate the event system
 */
export const EventsDemo: React.FC = () => {
  // Get events from the system category
  const systemEvents = useEvents<{ message?: string; appVersion?: string }>(EventCategory.SYSTEM)

  // Get events from the adapter category
  const adapterEvents = useEvents(EventCategory.ADAPTER)

  // Create a publisher for system info events
  const publishSystemInfo = useEventPublisher(EventCategory.SYSTEM, 'info', 'events-demo')

  // Input state for custom event
  const [message, setMessage] = useState('')

  // Publish a demo event when component mounts
  useEffect(() => {
    // Use the renderer event bus directly
    rendererEventBus.publish(new SystemInfoEvent('EventsDemo component mounted'))

    // Cleanup when unmounting
    return () => {
      rendererEventBus.publish(new SystemInfoEvent('EventsDemo component unmounted'))
    }
  }, [])

  // Handle form submission
  const handleSubmit = (e: React.FormEvent): void => {
    e.preventDefault()
    if (message.trim()) {
      // Use the publisher from the hook
      publishSystemInfo({ message })
      setMessage('')
    }
  }

  return (
    <div className="m-6 max-w-3xl mx-auto">
      <h2 className="text-2xl font-bold mb-6">Event System Demo</h2>

      <div className="mb-8 p-4 bg-white rounded shadow">
        <h3 className="text-lg font-medium mb-2">Publish Event</h3>
        <form onSubmit={handleSubmit} className="flex gap-2">
          <input
            type="text"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder="Enter message for system info event"
            className="flex-1 px-3 py-2 border border-gray-300 rounded"
          />
          <button type="submit" className="bg-blue-500 text-white px-4 py-2 rounded">
            Publish Event
          </button>
        </form>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white rounded shadow p-4">
          <h3 className="text-lg font-medium mb-2">System Events ({systemEvents.length})</h3>
          <div className="max-h-80 overflow-y-auto border border-gray-200 rounded">
            <ul className="divide-y divide-gray-200">
              {systemEvents.map((event) => (
                <li key={event.id} className="p-3 hover:bg-gray-50">
                  <div className="text-xs text-gray-500">
                    {new Date(event.timestamp).toLocaleTimeString()}
                  </div>
                  <div className="text-sm">
                    {event.payload && typeof event.payload === 'object' ? (
                      event.payload.message ? (
                        <span>{event.payload.message}</span>
                      ) : event.payload.appVersion ? (
                        <span>System started (v{event.payload.appVersion})</span>
                      ) : (
                        <span>{JSON.stringify(event.payload)}</span>
                      )
                    ) : (
                      <span>{String(event.payload)}</span>
                    )}
                  </div>
                </li>
              ))}
              {systemEvents.length === 0 && (
                <li className="p-3 text-sm text-gray-500">No events yet</li>
              )}
            </ul>
          </div>
        </div>

        <div className="bg-white rounded shadow p-4">
          <h3 className="text-lg font-medium mb-2">Adapter Events ({adapterEvents.length})</h3>
          <div className="max-h-80 overflow-y-auto border border-gray-200 rounded">
            <ul className="divide-y divide-gray-200">
              {adapterEvents.map((event) => (
                <li key={event.id} className="p-3 hover:bg-gray-50">
                  <div className="text-xs text-gray-500 flex justify-between">
                    <span>{new Date(event.timestamp).toLocaleTimeString()}</span>
                    <span className="font-medium">{event.source}</span>
                  </div>
                  <div className="text-sm">
                    <span className="inline-block px-2 py-1 mr-2 bg-gray-100 text-xs rounded">
                      {event.type || 'unknown'}
                    </span>
                    <span>
                      {event.payload
                        ? typeof event.payload === 'object'
                          ? JSON.stringify(event.payload)
                          : String(event.payload)
                        : 'No payload'}
                    </span>
                  </div>
                </li>
              ))}
              {adapterEvents.length === 0 && (
                <li className="p-3 text-sm text-gray-500">No events yet</li>
              )}
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}

export default EventsDemo
