import { useEffect, useState } from 'react'
import { AdapterStatusList } from './adapter-status-list'
import { EventBusStats } from './event-bus-stats'
import { WebSocketInfo } from './websocket-info'

export function Dashboard() {
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    // Simulate data loading
    const timer = setTimeout(() => {
      setIsLoading(false)
    }, 500)

    return () => clearTimeout(timer)
  }, [])

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary"></div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Dashboard</h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <AdapterStatusList />
        <div className="space-y-6">
          <EventBusStats />
          <WebSocketInfo />
        </div>
      </div>
    </div>
  )
}