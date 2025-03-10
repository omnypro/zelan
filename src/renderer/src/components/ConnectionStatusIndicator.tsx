import React from 'react'
import { useReconnection } from '@r/hooks'
import { AdapterStatus as AdapterStatusEnum } from '@s/adapters/interfaces/AdapterStatus'

export interface ConnectionStatusIndicatorProps {
  adapterId: string
  adapterName: string
  status: AdapterStatusEnum | { status: string; message?: string; timestamp: number }
  onReconnect?: () => Promise<void>
}

/**
 * Component that displays connection status and reconnection state
 * with ability to manually reconnect
 */
export const ConnectionStatusIndicator: React.FC<ConnectionStatusIndicatorProps> = ({
  adapterId,
  adapterName,
  status,
  onReconnect
}) => {
  const { reconnectionState, reconnectNow, isLoading, error } = useReconnection(adapterId)

  // Status badge component
  const StatusBadge: React.FC<{ status: ConnectionStatusIndicatorProps['status'] }> = ({ status }) => {
    let color = ''
    let displayStatus = ''

    // Extract the status string from either an enum value or an object
    const statusValue = typeof status === 'object' && status !== null ? status.status : status

    // Determine appropriate color and label based on the status value
    switch (statusValue) {
      case AdapterStatusEnum.CONNECTED:
      case 'connected':
        color = 'bg-green-100 text-green-800'
        displayStatus = 'Connected'
        break
      case AdapterStatusEnum.CONNECTING:
      case 'connecting':
        color = 'bg-yellow-100 text-yellow-800'
        displayStatus = 'Connecting'
        break
      case AdapterStatusEnum.RECONNECTING:
      case 'reconnecting':
        color = 'bg-yellow-100 text-yellow-800'
        displayStatus = 'Reconnecting'
        break
      case AdapterStatusEnum.ERROR:
      case 'error':
        color = 'bg-red-100 text-red-800'
        displayStatus = 'Error'
        break
      case AdapterStatusEnum.DISCONNECTED:
      case 'disconnected':
        color = 'bg-gray-100 text-gray-800'
        displayStatus = 'Disconnected'
        break
      default:
        color = 'bg-gray-100 text-gray-800'
        displayStatus = `Unknown (${statusValue})`
    }

    return <span className={`inline-block px-2 py-1 rounded-full text-xs font-medium ${color}`}>{displayStatus}</span>
  }

  // Format time from milliseconds timestamp
  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString()
  }

  // Handle reconnect button click
  const handleReconnect = async () => {
    try {
      if (onReconnect) {
        await onReconnect()
      } else {
        await reconnectNow()
      }
    } catch (err) {
      console.error('Failed to reconnect:', err)
    }
  }

  return (
    <div className="flex flex-col bg-white rounded-lg p-4 shadow">
      <div className="flex justify-between items-center mb-2">
        <h3 className="font-medium text-gray-800">{adapterName}</h3>
        <StatusBadge status={status} />
      </div>

      {/* Status message if it exists */}
      {typeof status === 'object' && status !== null && status.message && (
        <div className="mt-2 text-sm text-gray-700">{status.message}</div>
      )}

      {/* Reconnection state indicators */}
      {reconnectionState && (
        <div className="mt-2 space-y-1 text-sm">
          {reconnectionState.attempts > 0 && (
            <div className="flex justify-between">
              <span className="text-gray-600">Reconnection attempts:</span>
              <span className="font-medium">{reconnectionState.attempts}</span>
            </div>
          )}

          {reconnectionState.lastAttempt && reconnectionState.lastAttempt > 0 && (
            <div className="flex justify-between">
              <span className="text-gray-600">Last attempt:</span>
              <span className="font-medium">{formatTime(reconnectionState.lastAttempt)}</span>
            </div>
          )}

          {reconnectionState.pending && (
            <div className="flex items-center text-yellow-600 mt-1">
              <svg
                className="animate-spin -ml-1 mr-2 h-4 w-4 text-yellow-600"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              <span>Reconnection scheduled...</span>
            </div>
          )}
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="mt-2 text-sm text-red-600">
          <p>Error: {error.message}</p>
        </div>
      )}

      {/* Action buttons */}
      <div className="mt-3 flex justify-end space-x-2">
        {/* Extract status value for comparison */}
        {(() => {
          const statusValue = typeof status === 'object' && status !== null ? status.status : status

          return statusValue === 'error' || statusValue === 'disconnected' ? (
            <button
              className="px-3 py-1 text-sm font-medium rounded-md bg-blue-50 text-blue-700 hover:bg-blue-100 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
              onClick={handleReconnect}
              disabled={isLoading}
            >
              {isLoading ? (
                <span className="flex items-center">
                  <svg
                    className="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    ></circle>
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    ></path>
                  </svg>
                  Reconnecting...
                </span>
              ) : (
                'Reconnect'
              )}
            </button>
          ) : null
        })()}
      </div>
    </div>
  )
}

export default ConnectionStatusIndicator
