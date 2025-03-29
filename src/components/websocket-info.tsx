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
      <div className="animate-pulse rounded-lg p-4 shadow">
        <div className="h-16 rounded bg-gray-200"></div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="rounded-lg border-l-4 border-red-500 p-4 shadow">
        <p className="text-red-500">Error loading WebSocket info</p>
      </div>
    )
  }

  return (
    <div className="mx-4 rounded-lg p-4 text-sm shadow ring-1 ring-gray-300">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <div className={`h-3 w-3 rounded-full ${localData.active ? 'bg-green-500' : 'bg-red-500'}`}></div>
          <span className="font-medium">{localData.active ? 'Active' : 'Inactive'}</span>
        </div>

        <div className="text-sm text-gray-500">Port: {localData.port}</div>

        <div className="ml-auto flex items-center gap-1 text-sm">
          <span>{localData.connections}</span>
          <span className="">/</span>
          <span className="">{localData.maxConnections}</span>
          <span className="">connections</span>
        </div>
      </div>

      <div className="mt-3 text-sm">
        <div className="rounded p-2 font-mono select-all">ws://localhost:{localData.port}</div>
      </div>
    </div>
  )
}
