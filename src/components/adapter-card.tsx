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
    <div className="rounded-lg transition-colors">
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-full">
          {adapter.icon ? (
            <img src={adapter.icon} alt={adapter.name} className="h-6 w-6" />
          ) : (
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-blue-100 font-medium text-blue-600">
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
          {adapter.statusDetails && <p className="mt-1 text-xs text-gray-500">{adapter.statusDetails}</p>}
        </div>

        <div>
          {adapter.status === 'connected' && (
            <button
              onClick={onDisconnect}
              className="rounded border border-gray-300 px-3 py-1 text-sm hover:bg-gray-100">
              Disconnect
            </button>
          )}

          {(adapter.status === 'disconnected' || adapter.status === 'error') && (
            <button
              onClick={onConnect}
              className="rounded bg-blue-600 px-3 py-1 text-sm text-white hover:bg-blue-700"
              disabled={isDisabled}>
              Connect
            </button>
          )}

          {adapter.status === 'disabled' && (
            <button className="cursor-not-allowed rounded border border-gray-300 px-3 py-1 text-sm opacity-50" disabled>
              Disabled
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
