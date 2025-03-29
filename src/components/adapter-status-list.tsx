import { useMemo } from 'react'
import { AdapterCard } from './adapter-card'
import { useAdapterControl } from '../hooks/use-adapter-control'

export function AdapterStatusList() {
  // In a real implementation, this would be replaced with actual data
  const { adapters, connect, disconnect, loading } = useAdapterControl()

  // Organize adapters by status
  const organizedAdapters = useMemo(() => {
    const connected = adapters.filter((a) => a.status === 'connected')
    const disconnected = adapters.filter((a) => a.status === 'disconnected')
    const error = adapters.filter((a) => a.status === 'error')
    const disabled = adapters.filter((a) => a.status === 'disabled')

    return { connected, disconnected, error, disabled }
  }, [adapters])

  if (loading) {
    return (
      <div className="rounded-lg p-4 shadow">
        <h3 className="mb-4 text-lg font-medium">Adapters</h3>
        <div className="grid grid-cols-1 gap-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="flex animate-pulse space-x-4 rounded border p-4">
              <div className="h-10 w-10 rounded-full bg-gray-200"></div>
              <div className="flex-1 space-y-2">
                <div className="h-4 w-3/4 rounded bg-gray-200"></div>
                <div className="h-3 w-1/2 rounded bg-gray-200"></div>
              </div>
            </div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="flex">
      {organizedAdapters.connected.length > 0 && (
        <div className="grid grid-cols-1 gap-3">
          {organizedAdapters.connected.map((adapter) => (
            <AdapterCard
              key={adapter.id}
              adapter={adapter}
              onConnect={() => connect(adapter.id)}
              onDisconnect={() => disconnect(adapter.id)}
            />
          ))}
        </div>
      )}

      {organizedAdapters.disconnected.length > 0 && (
        <div className="grid grid-cols-1 gap-3">
          {organizedAdapters.disconnected.map((adapter) => (
            <AdapterCard
              key={adapter.id}
              adapter={adapter}
              onConnect={() => connect(adapter.id)}
              onDisconnect={() => disconnect(adapter.id)}
            />
          ))}
        </div>
      )}

      {organizedAdapters.error.length > 0 && (
        <div className="grid grid-cols-1 gap-3">
          {organizedAdapters.error.map((adapter) => (
            <AdapterCard
              key={adapter.id}
              adapter={adapter}
              onConnect={() => connect(adapter.id)}
              onDisconnect={() => disconnect(adapter.id)}
            />
          ))}
        </div>
      )}

      {organizedAdapters.disabled.length > 0 && (
        <div className="grid grid-cols-1 gap-3">
          {organizedAdapters.disabled.map((adapter) => (
            <AdapterCard
              key={adapter.id}
              adapter={adapter}
              onConnect={() => connect(adapter.id)}
              onDisconnect={() => disconnect(adapter.id)}
            />
          ))}
        </div>
      )}
    </div>
  )
}
