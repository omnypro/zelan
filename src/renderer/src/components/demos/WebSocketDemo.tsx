import React, { useState, useEffect } from 'react'

interface WebSocketStatus {
  running: boolean
  clientCount: number
  port: number
}

export const WebSocketDemo: React.FC = () => {
  const [status, setStatus] = useState<WebSocketStatus | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Fetch WebSocket server status
  const fetchStatus = async () => {
    try {
      setLoading(true)
      setError(null)

      // Check if websocket namespace exists
      if (!window.trpc || !window.trpc.websocket || !window.trpc.websocket.getStatus) {
        throw new Error('WebSocket tRPC endpoints not available')
      }

      const result = await window.trpc.websocket.getStatus.query()
      setStatus(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch WebSocket status')
      console.error('Error fetching WebSocket status:', err)
    } finally {
      setLoading(false)
    }
  }

  // Toggle WebSocket server state
  const toggleServer = async () => {
    try {
      setLoading(true)
      setError(null)

      // Check if websocket namespace exists
      if (!window.trpc || !window.trpc.websocket) {
        throw new Error('WebSocket tRPC endpoints not available')
      }

      if (status?.running) {
        // Check if stop endpoint exists
        if (!window.trpc.websocket.stop) {
          throw new Error('WebSocket stop endpoint not available')
        }

        // Stop server
        await window.trpc.websocket.stop.mutate()
      } else {
        // Check if start endpoint exists
        if (!window.trpc.websocket.start) {
          throw new Error('WebSocket start endpoint not available')
        }

        // Start server
        await window.trpc.websocket.start.mutate()
      }

      // Refresh status
      await fetchStatus()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to toggle WebSocket server')
      console.error('Error toggling WebSocket server:', err)
    } finally {
      setLoading(false)
    }
  }

  // Fetch status on component mount
  useEffect(() => {
    // Initial fetch
    fetchStatus().catch((err) => {
      console.error('Error in initial fetchStatus:', err)
    })

    // Set up interval to refresh status every 5 seconds
    const interval = setInterval(() => {
      fetchStatus().catch((err) => {
        console.error('Error in interval fetchStatus:', err)
      })
    }, 5000)

    // Clean up interval on unmount
    return () => clearInterval(interval)
  }, [])

  // Function to check if tRPC is available
  const isTrpcAvailable = () => {
    return !!(window.trpc && window.trpc.websocket)
  }

  return (
    <div className="p-4 border rounded-lg shadow-sm">
      <h2 className="text-xl font-semibold mb-4">WebSocket Server</h2>

      {!isTrpcAvailable() ? (
        <div className="bg-yellow-100 border border-yellow-400 text-yellow-700 px-4 py-3 rounded mb-4">
          <p className="font-medium">tRPC WebSocket endpoints not available</p>
          <p className="text-sm mt-1">
            The WebSocket server control functionality is not available. This may be because the
            tRPC client is still initializing or the WebSocket endpoints are not properly
            registered.
          </p>
          <button
            onClick={fetchStatus}
            className="mt-2 px-3 py-1 bg-yellow-200 hover:bg-yellow-300 rounded-md text-sm font-medium"
          >
            Retry
          </button>
        </div>
      ) : (
        <>
          {loading && !status && (
            <div className="flex items-center justify-center h-20">
              <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-blue-500"></div>
            </div>
          )}

          {error && (
            <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
              <p>{error}</p>
            </div>
          )}

          {status && (
            <>
              <div className="mb-4 grid grid-cols-2 gap-4">
                <div>
                  <span className="font-medium">Status:</span>
                  <span
                    className={`ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                      status.running ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
                    }`}
                  >
                    {status.running ? 'Running' : 'Stopped'}
                  </span>
                </div>

                <div>
                  <span className="font-medium">Port:</span>
                  <span className="ml-2">{status.port}</span>
                </div>

                <div>
                  <span className="font-medium">Connected Clients:</span>
                  <span className="ml-2">{status.clientCount}</span>
                </div>
              </div>

              <div className="flex justify-between items-center mt-4">
                <button
                  onClick={toggleServer}
                  disabled={loading}
                  className={`px-4 py-2 rounded-md text-white font-medium ${
                    status.running
                      ? 'bg-red-500 hover:bg-red-600'
                      : 'bg-green-500 hover:bg-green-600'
                  } disabled:opacity-50 disabled:cursor-not-allowed`}
                >
                  {loading ? (
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
                      Processing...
                    </span>
                  ) : status.running ? (
                    'Stop Server'
                  ) : (
                    'Start Server'
                  )}
                </button>

                <button
                  onClick={fetchStatus}
                  disabled={loading}
                  className="px-3 py-1 border border-gray-300 rounded-md text-sm hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Refresh
                </button>
              </div>

              {status.running && (
                <div className="mt-6 border-t pt-4">
                  <h3 className="text-sm font-medium mb-2">Connection Information</h3>
                  <div className="bg-gray-100 p-3 rounded font-mono text-sm">
                    <p>
                      WebSocket URL:{' '}
                      <span className="text-blue-600">ws://localhost:{status.port}</span>
                    </p>
                  </div>
                  <p className="text-xs text-gray-500 mt-2">
                    See the WebSocket API documentation for more details on how to use the WebSocket
                    server.
                  </p>
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
  )
}
