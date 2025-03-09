import React from 'react'
import { useTrpcAdapters, useAdapterControl } from '@r/hooks/useTrpc'
import { ConnectionStatusIndicator } from './ConnectionStatusIndicator'

/**
 * Enhanced adapter status component that shows connection status with reconnection capabilities
 */
export const EnhancedAdapterStatus: React.FC = () => {
  const adapters = useTrpcAdapters()

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4">Adapter Connection Status</h2>

      {adapters.length === 0 ? (
        <p className="text-gray-500">No adapters available</p>
      ) : (
        <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
          {adapters.map((adapter) => (
            <AdapterStatusCard key={adapter.id} adapterId={adapter.id} />
          ))}
        </div>
      )}
    </div>
  )
}

/**
 * Individual adapter status card
 */
interface AdapterStatusCardProps {
  adapterId: string
}

const AdapterStatusCard: React.FC<AdapterStatusCardProps> = ({ adapterId }) => {
  const adapters = useTrpcAdapters()
  const { startAdapter, stopAdapter, isLoading } = useAdapterControl(adapterId)

  // Find the adapter in the list
  const adapter = adapters.find((a) => a.id === adapterId)

  if (!adapter) {
    return null
  }

  // Handle manual reconnection
  const handleReconnect = async () => {
    // Extract status value, handling both string and object formats
    const statusValue = typeof adapter.status === 'object' && adapter.status !== null
      ? adapter.status.status
      : adapter.status
      
    if (statusValue === 'disconnected' || statusValue === 'error') {
      await startAdapter()
    }
  }

  return (
    <ConnectionStatusIndicator
      adapterId={adapter.id}
      adapterName={adapter.name}
      status={adapter.status}
      onReconnect={handleReconnect}
    />
  )
}

export default EnhancedAdapterStatus