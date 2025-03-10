import React from 'react'
import { EventCategory, AdapterEventType } from '@s/types/events'
import { useEvents } from '@r/hooks/useEventStream'
import { AdapterStatus as AdapterStatusEnum } from '@s/adapters/interfaces/AdapterStatus'

/**
 * Component to display adapter status information
 */
export const AdapterStatus: React.FC = () => {
  // Get adapter status events
  const adapterStatusEvents = useEvents(EventCategory.ADAPTER, AdapterEventType.STATUS)

  // Group by adapter id
  const adapterStatuses = adapterStatusEvents.reduce(
    (acc, event) => {
      // Make sure data is properly formatted before accessing properties
      const data = event.data || {}

      // Extract properties safely
      const id = typeof data.id === 'string' ? data.id : 'unknown'
      const name = typeof data.name === 'string' ? data.name : 'Unknown Adapter'
      const type = typeof data.type === 'string' ? data.type : 'unknown'

      // Handle status which might be either an object with status property or a direct status value
      let statusValue
      let statusMessage

      if (data.status && typeof data.status === 'object') {
        // Handle case where status is an object with status and message properties
        statusValue = data.status.status
        statusMessage = data.status.message
      } else if (typeof data.status === 'string') {
        // Handle case where status is just a string
        statusValue = data.status
        statusMessage = data.message
      } else {
        // Default status
        statusValue = AdapterStatusEnum.DISCONNECTED
        statusMessage = 'Unknown status'
      }

      // Only add if we don't already have a more recent status for this adapter
      if (!acc[id] || acc[id].timestamp < event.timestamp) {
        acc[id] = {
          id,
          name,
          type,
          status: statusValue,
          message: statusMessage,
          timestamp: event.timestamp
        }
      }

      return acc
    },
    {} as Record<
      string,
      {
        id: string
        name: string
        type: string
        status: AdapterStatusEnum
        message?: string
        timestamp: number
      }
    >
  )

  // Convert to array and sort by name
  const adapters = Object.values(adapterStatuses).sort((a, b) => a.name.localeCompare(b.name))

  // Status badge component
  const StatusBadge: React.FC<{ status: AdapterStatusEnum }> = ({ status }) => {
    let color = ''
    let displayStatus = ''

    // Handle when status is an object rather than enum string
    const statusValue = typeof status === 'object' && status !== null ? JSON.stringify(status) : String(status)

    switch (statusValue) {
      case AdapterStatusEnum.CONNECTED:
        color = 'bg-green-100 text-green-800'
        displayStatus = 'Connected'
        break
      case AdapterStatusEnum.CONNECTING:
        color = 'bg-yellow-100 text-yellow-800'
        displayStatus = 'Connecting'
        break
      case AdapterStatusEnum.RECONNECTING:
        color = 'bg-yellow-100 text-yellow-800'
        displayStatus = 'Reconnecting'
        break
      case AdapterStatusEnum.ERROR:
        color = 'bg-red-100 text-red-800'
        displayStatus = 'Error'
        break
      case AdapterStatusEnum.DISCONNECTED:
        color = 'bg-gray-100 text-gray-800'
        displayStatus = 'Disconnected'
        break
      default:
        color = 'bg-gray-100 text-gray-800'
        displayStatus = 'Unknown'
    }

    return <span className={`inline-block px-2 py-1 rounded-full text-xs font-medium ${color}`}>{displayStatus}</span>
  }

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4">Adapter Status</h2>

      {adapters.length === 0 ? (
        <p className="text-gray-500">No adapters available</p>
      ) : (
        <div className="grid gap-4 grid-cols-1 md:grid-cols-2">
          {adapters.map((adapter) => (
            <div key={adapter.id} className="bg-white rounded-lg shadow p-4">
              <div className="flex justify-between items-center mb-2">
                <h3 className="font-medium">{adapter.name}</h3>
                <StatusBadge status={adapter.status} />
              </div>

              <div className="text-sm text-gray-500 mb-1">Type: {adapter.type}</div>

              {adapter.message && <div className="text-sm mt-2">{adapter.message}</div>}

              <div className="text-xs text-gray-400 mt-2">
                Last updated: {new Date(adapter.timestamp).toLocaleTimeString()}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default AdapterStatus
