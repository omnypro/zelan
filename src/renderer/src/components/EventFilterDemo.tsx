import { useState, useEffect } from 'react'
import { EventCategory } from '@s/types/events'
import { useFilteredEvents, useEventPublisher } from '@r/hooks/useEventStream'
import { EventFilterCriteria } from '@s/core/bus'

export default function EventFilterDemo() {
  // State for filter settings
  const [category, setCategory] = useState<EventCategory>(EventCategory.SYSTEM)
  const [type, setType] = useState<string>('')
  const [sourceId, setSourceId] = useState<string>('')
  const [sourceType, setSourceType] = useState<string>('')
  const [sinceMinutes, setSinceMinutes] = useState<number>(60)
  const [showAdvanced, setShowAdvanced] = useState(false)

  // Create combined filter criteria
  const [filterCriteria, setFilterCriteria] = useState<EventFilterCriteria>({})

  // Event publisher for testing
  const publishTestEvent = useEventPublisher(EventCategory.USER, 'filter-demo', 'filter-demo')

  // Update filter criteria when any filter option changes
  useEffect(() => {
    const criteria: EventFilterCriteria = {}

    if (category) {
      criteria.category = category
    }

    if (type) {
      criteria.type = type
    }

    if (sourceId) {
      criteria.sourceId = sourceId
    }

    if (sourceType) {
      criteria.sourceType = sourceType
    }

    if (sinceMinutes > 0) {
      criteria.since = Date.now() - sinceMinutes * 60 * 1000
    }

    setFilterCriteria(criteria)
  }, [category, type, sourceId, sourceType, sinceMinutes])

  // Get filtered events using our new hook
  const filteredEvents = useFilteredEvents(filterCriteria)

  // Generate a test event
  const handleTestEvent = () => {
    publishTestEvent({
      message: 'Test event from filter demo',
      timestamp: Date.now(),
      random: Math.random()
    })
  }

  return (
    <div className="p-4 space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">Event Filter Demo</h2>
        <button onClick={handleTestEvent} className="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
          Generate Test Event
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
        <div className="space-y-4 p-4 border rounded">
          <h3 className="text-xl font-semibold">Filter Settings</h3>

          <div>
            <label className="block text-sm font-medium mb-1">Category</label>
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value as EventCategory)}
              className="w-full p-2 border rounded"
            >
              <option value="">Any Category</option>
              {Object.values(EventCategory).map((cat) => (
                <option key={cat} value={cat}>
                  {cat}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium mb-1">Type</label>
            <input
              type="text"
              value={type}
              onChange={(e) => setType(e.target.value)}
              placeholder="Event type"
              className="w-full p-2 border rounded"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1">Since (minutes ago)</label>
            <input
              type="number"
              value={sinceMinutes}
              onChange={(e) => setSinceMinutes(Number(e.target.value))}
              min="1"
              max="1440"
              className="w-full p-2 border rounded"
            />
          </div>

          <div>
            <button onClick={() => setShowAdvanced(!showAdvanced)} className="text-blue-500 text-sm">
              {showAdvanced ? 'Hide' : 'Show'} Advanced Filters
            </button>
          </div>

          {showAdvanced && (
            <>
              <div>
                <label className="block text-sm font-medium mb-1">Source ID</label>
                <input
                  type="text"
                  value={sourceId}
                  onChange={(e) => setSourceId(e.target.value)}
                  placeholder="Source ID"
                  className="w-full p-2 border rounded"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">Source Type</label>
                <input
                  type="text"
                  value={sourceType}
                  onChange={(e) => setSourceType(e.target.value)}
                  placeholder="Source type"
                  className="w-full p-2 border rounded"
                />
              </div>
            </>
          )}

          <div className="mt-4 p-3 bg-gray-100 rounded overflow-auto max-h-32">
            <pre className="text-xs">{JSON.stringify(filterCriteria, null, 2)}</pre>
          </div>
        </div>

        <div className="space-y-4">
          <h3 className="text-xl font-semibold">Filtered Events ({filteredEvents.length})</h3>

          <div className="h-96 overflow-auto border rounded p-2">
            {filteredEvents.length === 0 ? (
              <div className="text-gray-500 p-4 text-center">No events match the current filters</div>
            ) : (
              filteredEvents.map((event) => (
                <div key={event.id} className="p-2 mb-2 border-b">
                  <div className="text-sm font-medium">{new Date(event.timestamp).toLocaleTimeString()}</div>
                  <pre className="text-xs mt-1 overflow-auto max-h-32">{JSON.stringify(event.data, null, 2)}</pre>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
