import React, { useState, useEffect } from 'react'

interface AdapterCreationFormProps {
  onCreateAdapter: (config: any) => Promise<void>
}

const AdapterCreationForm: React.FC<AdapterCreationFormProps> = ({ onCreateAdapter }) => {
  const [adapterType, setAdapterType] = useState<string>('test')
  const [name, setName] = useState<string>('')
  const [isLoading, setIsLoading] = useState<boolean>(false)
  const [error, setError] = useState<string | null>(null)

  // OBS specific options
  const [obsHost, setObsHost] = useState<string>('localhost')
  const [obsPort, setObsPort] = useState<number>(4455)
  const [obsPassword, setObsPassword] = useState<string>('')
  const [autoReconnect, setAutoReconnect] = useState<boolean>(true)

  // Test adapter options
  const [eventInterval, setEventInterval] = useState<number>(3000)
  const [simulateErrors, setSimulateErrors] = useState<boolean>(false)
  
  // Twitch adapter options
  const [twitchPollInterval, setTwitchPollInterval] = useState<number>(60000)
  const [twitchIncludeSubscriptions, setTwitchIncludeSubscriptions] = useState<boolean>(false)

  // Generate a unique ID for the adapter
  const generateAdapterId = () => {
    return `${adapterType}-${Date.now()}-${Math.floor(Math.random() * 1000)}`
  }

  // Form submission handler
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setIsLoading(true)
    setError(null)

    try {
      const id = generateAdapterId()
      let options: any = {}

      // Configure options based on adapter type
      if (adapterType === 'obs') {
        options = {
          host: obsHost,
          port: obsPort,
          password: obsPassword || undefined,
          autoReconnect,
          reconnectInterval: 5000
        }
      } else if (adapterType === 'test') {
        options = {
          eventInterval,
          simulateErrors,
          eventTypes: ['message', 'follow', 'subscription']
        }
      } else if (adapterType === 'twitch') {
        options = {
          channelName: '', // Will use authenticated user's channel by default
          pollInterval: twitchPollInterval,
          includeSubscriptions: twitchIncludeSubscriptions,
          eventsToTrack: [
            'channel.update', 
            'stream.online', 
            'stream.offline'
          ]
        }
      }

      // Create the adapter config
      const adapterConfig = {
        id,
        type: adapterType,
        name: name || `${adapterType.charAt(0).toUpperCase() + adapterType.slice(1)} Adapter`,
        enabled: true,
        options
      }

      await onCreateAdapter(adapterConfig)

      // Reset form
      setName('')

      if (adapterType === 'obs') {
        setObsHost('localhost')
        setObsPort(4455)
        setObsPassword('')
      } else if (adapterType === 'test') {
        setEventInterval(3000)
        setSimulateErrors(false)
      } else if (adapterType === 'twitch') {
        setTwitchPollInterval(60000)
        setTwitchIncludeSubscriptions(false)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create adapter')
      console.error('Error creating adapter:', err)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h3 className="text-lg font-semibold mb-4">Create New Adapter</h3>

      {error && (
        <div className="mb-4 bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
          <p>{error}</p>
        </div>
      )}

      <form onSubmit={handleSubmit}>
        <div className="mb-4">
          <label className="block text-sm font-medium mb-1">Adapter Type</label>
          <select
            value={adapterType}
            onChange={(e) => setAdapterType(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
            disabled={isLoading}
          >
            <option value="test">Test Adapter</option>
            <option value="obs">OBS Adapter</option>
            <option value="twitch">Twitch Adapter</option>
          </select>
        </div>

        <div className="mb-4">
          <label className="block text-sm font-medium mb-1">Adapter Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={`${adapterType.charAt(0).toUpperCase() + adapterType.slice(1)} Adapter`}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
            disabled={isLoading}
          />
        </div>

        {/* OBS specific options */}
        {adapterType === 'obs' && (
          <div className="bg-gray-50 p-4 rounded-md mb-4">
            <h4 className="font-medium mb-3">OBS WebSocket Settings</h4>

            <div className="mb-3">
              <label className="block text-sm font-medium mb-1">Host</label>
              <input
                type="text"
                value={obsHost}
                onChange={(e) => setObsHost(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                disabled={isLoading}
                placeholder="localhost"
              />
            </div>

            <div className="mb-3">
              <label className="block text-sm font-medium mb-1">Port</label>
              <input
                type="number"
                value={obsPort}
                onChange={(e) => setObsPort(parseInt(e.target.value, 10))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                disabled={isLoading}
                placeholder="4455"
              />
            </div>

            <div className="mb-3">
              <label className="block text-sm font-medium mb-1">Password (optional)</label>
              <input
                type="password"
                value={obsPassword}
                onChange={(e) => setObsPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                disabled={isLoading}
                placeholder="Leave empty if no password"
              />
            </div>

            <div className="flex items-center">
              <input
                type="checkbox"
                id="autoReconnect"
                checked={autoReconnect}
                onChange={(e) => setAutoReconnect(e.target.checked)}
                className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                disabled={isLoading}
              />
              <label htmlFor="autoReconnect" className="ml-2 block text-sm text-gray-900">
                Auto-reconnect if connection is lost
              </label>
            </div>
          </div>
        )}

        {/* Test adapter options */}
        {adapterType === 'test' && (
          <div className="bg-gray-50 p-4 rounded-md mb-4">
            <h4 className="font-medium mb-3">Test Adapter Settings</h4>

            <div className="mb-3">
              <label className="block text-sm font-medium mb-1">Event Interval (ms)</label>
              <input
                type="number"
                value={eventInterval}
                onChange={(e) => setEventInterval(parseInt(e.target.value, 10))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                disabled={isLoading}
                min={500}
                max={30000}
                step={500}
              />
              <p className="text-xs text-gray-500 mt-1">
                How often test events will be generated (in milliseconds)
              </p>
            </div>

            <div className="flex items-center">
              <input
                type="checkbox"
                id="simulateErrors"
                checked={simulateErrors}
                onChange={(e) => setSimulateErrors(e.target.checked)}
                className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                disabled={isLoading}
              />
              <label htmlFor="simulateErrors" className="ml-2 block text-sm text-gray-900">
                Simulate random errors for testing
              </label>
            </div>
          </div>
        )}
        
        {/* Twitch adapter options */}
        {adapterType === 'twitch' && (
          <div className="bg-gray-50 p-4 rounded-md mb-4">
            <h4 className="font-medium mb-3">Twitch Adapter Settings</h4>
            
            <div className="mb-3">
              <label className="block text-sm font-medium mb-1">Poll Interval (ms)</label>
              <input
                type="number"
                value={twitchPollInterval}
                onChange={(e) => setTwitchPollInterval(parseInt(e.target.value, 10))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                disabled={isLoading}
                min={30000}
                max={300000}
                step={30000}
              />
              <p className="text-xs text-gray-500 mt-1">
                How often to poll for channel information (in milliseconds)
              </p>
            </div>
            
            <div className="flex items-center">
              <input
                type="checkbox"
                id="twitchIncludeSubscriptions"
                checked={twitchIncludeSubscriptions}
                onChange={(e) => setTwitchIncludeSubscriptions(e.target.checked)}
                className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                disabled={isLoading}
              />
              <label htmlFor="twitchIncludeSubscriptions" className="ml-2 block text-sm text-gray-900">
                Include subscription data in polls (requires additional scopes)
              </label>
            </div>
            
            <div className="mt-4 p-3 bg-blue-50 rounded text-sm text-blue-800 border border-blue-200">
              <p className="font-medium">Note:</p>
              <p>You must authenticate with Twitch before creating this adapter. The adapter will use the authenticated user's channel.</p>
            </div>
          </div>
        )}

        <div className="flex justify-end">
          <button
            type="submit"
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
            disabled={isLoading}
          >
            {isLoading ? 'Creating...' : 'Create Adapter'}
          </button>
        </div>
      </form>
    </div>
  )
}

export default AdapterCreationForm
