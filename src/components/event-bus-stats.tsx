import { useEffect, useState } from 'react'
import { useDataFetching } from '../hooks/use-data-fetching'

type EventBusStatsData = {
  totalEvents: number
  activeSubscribers: number
  eventsPerSecond: number
  eventsBySource: Record<string, number>
}

export function EventBusStats() {
  // In a real implementation, this would fetch data from Tauri
  const { data, isLoading, error } = useDataFetching<EventBusStatsData>('get_event_bus_status')
  
  // For demo purposes, we'll use mock data
  const [localData, setLocalData] = useState<EventBusStatsData>({
    totalEvents: 1245,
    activeSubscribers: 3,
    eventsPerSecond: 2.5,
    eventsBySource: {
      twitch: 750,
      obs: 380,
      test: 115
    }
  })

  useEffect(() => {
    if (data) {
      setLocalData(data)
    }
  }, [data])

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg p-4 shadow animate-pulse">
        <h3 className="text-lg font-medium mb-4">Event Bus</h3>
        <div className="grid grid-cols-2 gap-4">
          {[1, 2, 3, 4].map(i => (
            <div key={i} className="h-16 bg-gray-200 rounded"></div>
          ))}
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="bg-white rounded-lg p-4 shadow border-l-4 border-red-500">
        <h3 className="text-lg font-medium mb-2">Event Bus</h3>
        <p className="text-red-500">Error loading event bus stats</p>
      </div>
    )
  }

  return (
    <div className="bg-white rounded-lg p-4 shadow">
      <h3 className="text-lg font-medium mb-4">Event Bus</h3>
      
      <div className="grid grid-cols-2 gap-4">
        <div className="border rounded-lg p-3">
          <div className="text-3xl font-bold">{localData.totalEvents.toLocaleString()}</div>
          <div className="text-sm text-gray-500">Total Events</div>
        </div>
        
        <div className="border rounded-lg p-3">
          <div className="text-3xl font-bold">{localData.activeSubscribers}</div>
          <div className="text-sm text-gray-500">Active Subscribers</div>
        </div>
        
        <div className="border rounded-lg p-3">
          <div className="text-3xl font-bold">{localData.eventsPerSecond.toFixed(1)}</div>
          <div className="text-sm text-gray-500">Events/Second</div>
        </div>
        
        <div className="border rounded-lg p-3">
          <div className="text-3xl font-bold">{Object.keys(localData.eventsBySource).length}</div>
          <div className="text-sm text-gray-500">Active Sources</div>
        </div>
      </div>
      
      {Object.keys(localData.eventsBySource).length > 0 && (
        <div className="mt-4">
          <h4 className="text-sm font-medium text-gray-500 mb-2">Events by Source</h4>
          <div className="space-y-2">
            {Object.entries(localData.eventsBySource).map(([source, count]) => (
              <div key={source} className="flex items-center">
                <div className="w-24 truncate">{source}</div>
                <div className="flex-grow h-2 bg-gray-200 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-blue-600"
                    style={{
                      width: `${(count / localData.totalEvents) * 100}%`,
                    }}
                  ></div>
                </div>
                <div className="w-16 text-right text-sm">{count}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}