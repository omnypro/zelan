import { useMemo } from 'react'
import { AdapterCard } from './adapter-card'
import { useAdapterControl } from '../hooks/use-adapter-control'

export function AdapterStatusList() {
  // In a real implementation, this would be replaced with actual data
  const { adapters, connect, disconnect, loading } = useAdapterControl()
  
  // Organize adapters by status
  const organizedAdapters = useMemo(() => {
    const connected = adapters.filter(a => a.status === 'connected')
    const disconnected = adapters.filter(a => a.status === 'disconnected')
    const error = adapters.filter(a => a.status === 'error')
    const disabled = adapters.filter(a => a.status === 'disabled')
    
    return { connected, disconnected, error, disabled }
  }, [adapters])
  
  if (loading) {
    return (
      <div className="bg-white rounded-lg p-4 shadow">
        <h3 className="text-lg font-medium mb-4">Adapters</h3>
        <div className="grid grid-cols-1 gap-4">
          {[1, 2, 3].map(i => (
            <div key={i} className="animate-pulse flex space-x-4 border p-4 rounded">
              <div className="rounded-full bg-gray-200 h-10 w-10"></div>
              <div className="flex-1 space-y-2">
                <div className="h-4 bg-gray-200 rounded w-3/4"></div>
                <div className="h-3 bg-gray-200 rounded w-1/2"></div>
              </div>
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="bg-white rounded-lg p-4 shadow">
      <h3 className="text-lg font-medium mb-4">Adapters</h3>
      
      <div className="space-y-6">
        {organizedAdapters.connected.length > 0 && (
          <div>
            <h4 className="text-sm font-medium text-gray-500 mb-2">Connected</h4>
            <div className="grid grid-cols-1 gap-3">
              {organizedAdapters.connected.map(adapter => (
                <AdapterCard
                  key={adapter.id}
                  adapter={adapter}
                  onConnect={() => connect(adapter.id)}
                  onDisconnect={() => disconnect(adapter.id)}
                />
              ))}
            </div>
          </div>
        )}
        
        {organizedAdapters.disconnected.length > 0 && (
          <div>
            <h4 className="text-sm font-medium text-gray-500 mb-2">Disconnected</h4>
            <div className="grid grid-cols-1 gap-3">
              {organizedAdapters.disconnected.map(adapter => (
                <AdapterCard
                  key={adapter.id}
                  adapter={adapter}
                  onConnect={() => connect(adapter.id)}
                  onDisconnect={() => disconnect(adapter.id)}
                />
              ))}
            </div>
          </div>
        )}
        
        {organizedAdapters.error.length > 0 && (
          <div>
            <h4 className="text-sm font-medium text-gray-500 mb-2">Error</h4>
            <div className="grid grid-cols-1 gap-3">
              {organizedAdapters.error.map(adapter => (
                <AdapterCard
                  key={adapter.id}
                  adapter={adapter}
                  onConnect={() => connect(adapter.id)}
                  onDisconnect={() => disconnect(adapter.id)}
                />
              ))}
            </div>
          </div>
        )}
        
        {organizedAdapters.disabled.length > 0 && (
          <div>
            <h4 className="text-sm font-medium text-gray-500 mb-2">Disabled</h4>
            <div className="grid grid-cols-1 gap-3">
              {organizedAdapters.disabled.map(adapter => (
                <AdapterCard
                  key={adapter.id}
                  adapter={adapter}
                  onConnect={() => connect(adapter.id)}
                  onDisconnect={() => disconnect(adapter.id)}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}