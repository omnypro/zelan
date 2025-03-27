import { useEffect, useState } from 'react'
import { useDataFetching } from '../hooks/use-data-fetching'

type WebSocketInfoData = {
  port: number
  active: boolean
  connections: number
  maxConnections: number
}

export function WebSocketInfo() {
  // In a real implementation, this would fetch data from Tauri
  const { data, isLoading, error } = useDataFetching<WebSocketInfoData>('get_websocket_info')
  
  // For demo purposes, we'll use mock data
  const [localData, setLocalData] = useState<WebSocketInfoData>({
    port: 9000,
    active: true,
    connections: 2,
    maxConnections: 100
  })

  useEffect(() => {
    if (data) {
      setLocalData(data)
    }
  }, [data])

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg p-4 shadow animate-pulse">
        <h3 className="text-lg font-medium mb-4">WebSocket Server</h3>
        <div className="h-16 bg-gray-200 rounded"></div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="bg-white rounded-lg p-4 shadow border-l-4 border-red-500">
        <h3 className="text-lg font-medium mb-2">WebSocket Server</h3>
        <p className="text-red-500">Error loading WebSocket info</p>
      </div>
    )
  }

  return (
    <div className="bg-white rounded-lg p-4 shadow">
      <h3 className="text-lg font-medium mb-4">WebSocket Server</h3>
      
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <div
            className={`w-3 h-3 rounded-full ${localData.active ? 'bg-green-500' : 'bg-red-500'}`}
          ></div>
          <span className="font-medium">
            {localData.active ? 'Active' : 'Inactive'}
          </span>
        </div>
        
        <div className="text-sm text-gray-500">
          Port: {localData.port}
        </div>
        
        <div className="ml-auto flex items-center gap-1 text-sm">
          <span>{localData.connections}</span>
          <span className="text-gray-500">/</span>
          <span className="text-gray-500">{localData.maxConnections}</span>
          <span className="text-gray-500 ml-1">connections</span>
        </div>
      </div>
      
      <div className="mt-3 text-sm">
        <div className="bg-gray-100 p-2 rounded font-mono select-all">
          ws://localhost:{localData.port}
        </div>
      </div>
    </div>
  )
}