import { StatusIndicator } from './status-indicator'

type AdapterStatus = 'connected' | 'disconnected' | 'error' | 'disabled'

type Adapter = {
  id: string
  name: string
  description: string
  status: AdapterStatus
  statusDetails?: string
  icon?: string
}

type AdapterCardProps = {
  adapter: Adapter
  onConnect: () => void
  onDisconnect: () => void
}

export function AdapterCard({ adapter, onConnect, onDisconnect }: AdapterCardProps) {
  const isDisabled = adapter.status === 'disabled'
  
  return (
    <div className="border rounded-lg p-4 hover:bg-gray-50 transition-colors">
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center">
          {adapter.icon ? (
            <img src={adapter.icon} alt={adapter.name} className="w-6 h-6" />
          ) : (
            <div className="w-6 h-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center font-medium">
              {adapter.name.charAt(0)}
            </div>
          )}
        </div>
        
        <div className="flex-grow">
          <div className="flex items-center gap-2">
            <h3 className="font-medium">{adapter.name}</h3>
            <StatusIndicator status={adapter.status} />
          </div>
          <p className="text-sm text-gray-500">{adapter.description}</p>
          {adapter.statusDetails && (
            <p className="text-xs text-gray-500 mt-1">{adapter.statusDetails}</p>
          )}
        </div>
        
        <div>
          {adapter.status === 'connected' && (
            <button
              onClick={onDisconnect}
              className="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100"
            >
              Disconnect
            </button>
          )}
          
          {(adapter.status === 'disconnected' || adapter.status === 'error') && (
            <button
              onClick={onConnect}
              className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              disabled={isDisabled}
            >
              Connect
            </button>
          )}
          
          {adapter.status === 'disabled' && (
            <button
              className="px-3 py-1 text-sm border border-gray-300 rounded opacity-50 cursor-not-allowed"
              disabled
            >
              Disabled
            </button>
          )}
        </div>
      </div>
    </div>
  )
}