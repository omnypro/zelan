import { useState } from 'react'
import { EventCategory } from '@s/types/events'
import { formatDistanceToNow } from 'date-fns'

interface EventCardProps {
  id: string
  timestamp: number
  category?: EventCategory
  type?: string
  source?: {
    id: string
    type: string
  }
  payload: any
  expanded?: boolean
  onClick?: () => void
}

/**
 * Formats the payload for display based on its content and type
 */
function formatPayload(payload: any, type?: string): React.ReactNode {
  if (!payload) return <span className="text-gray-400 italic">No payload</span>

  // Handle string payloads
  if (typeof payload === 'string') {
    return <span>{payload}</span>
  }

  // Handle adapter status events specially
  if (type === 'status' && payload.status) {
    return (
      <div>
        <div className="flex gap-1 items-center">
          <StatusIndicator status={payload.status} />
          <span className="font-medium">{payload.status}</span>
        </div>
        {payload.message && <div className="text-gray-600">{payload.message}</div>}
      </div>
    )
  }

  // Handle message property if it exists
  if (payload.message) {
    return <span>{payload.message}</span>
  }

  // Try to find a good summary property
  const summaryProps = ['name', 'title', 'summary', 'description']
  for (const prop of summaryProps) {
    if (payload[prop] && typeof payload[prop] === 'string') {
      return <span>{payload[prop]}</span>
    }
  }

  // When all else fails, return the stringified object
  return <span className="font-mono text-xs whitespace-pre-wrap">{JSON.stringify(payload, null, 2)}</span>
}

/**
 * Status indicator for adapter status events
 */
function StatusIndicator({ status }: { status: string }) {
  let bgColor = 'bg-gray-400'

  switch (status.toLowerCase()) {
    case 'connected':
    case 'ready':
    case 'active':
      bgColor = 'bg-green-500'
      break
    case 'disconnected':
    case 'inactive':
      bgColor = 'bg-red-500'
      break
    case 'connecting':
    case 'initializing':
      bgColor = 'bg-yellow-500'
      break
    case 'error':
      bgColor = 'bg-red-600'
      break
  }

  return <div className={`w-2 h-2 rounded-full ${bgColor}`} />
}

/**
 * Event source badge
 */
function SourceBadge({ source }: { source: { id: string; type: string } }) {
  return <span className="inline-block px-2 py-0.5 text-xs bg-gray-100 rounded-full">{source.id}</span>
}

/**
 * Event type badge with category-specific colors
 */
function TypeBadge({ type, category }: { type?: string; category?: EventCategory }) {
  if (!type) return null

  let bgColor = 'bg-gray-100'

  if (category) {
    switch (category) {
      case EventCategory.SYSTEM:
        bgColor = 'bg-blue-100'
        break
      case EventCategory.ADAPTER:
        bgColor = 'bg-purple-100'
        break
      case EventCategory.AUTH:
        bgColor = 'bg-yellow-100'
        break
      case EventCategory.ERROR:
        bgColor = 'bg-red-100'
        break
      case EventCategory.TEST:
        bgColor = 'bg-green-100'
        break
    }
  }

  return <span className={`inline-block px-2 py-0.5 text-xs ${bgColor} rounded-md`}>{type}</span>
}

/**
 * Event card component displays a single event with expandable details
 */
export function EventCard({
  id,
  timestamp,
  category,
  type,
  source,
  payload,
  expanded: initialExpanded = false,
  onClick
}: EventCardProps) {
  const [expanded, setExpanded] = useState(initialExpanded)

  const timeFormatted = new Date(timestamp).toLocaleTimeString()
  const relativeTime = formatDistanceToNow(timestamp, { addSuffix: true })

  const handleExpand = () => {
    setExpanded(!expanded)
    if (onClick) onClick()
  }

  return (
    <div
      className={`p-3 border-b hover:bg-gray-50 transition-colors cursor-pointer ${expanded ? 'bg-gray-50' : ''}`}
      onClick={handleExpand}
    >
      <div className="flex justify-between items-start mb-1">
        <div className="flex gap-2 items-center">
          {category && <TypeBadge type={type} category={category} />}
          <span className="text-xs text-gray-500">{timeFormatted}</span>
          <span className="text-xs text-gray-400" title={new Date(timestamp).toLocaleString()}>
            ({relativeTime})
          </span>
        </div>

        {source && <SourceBadge source={source} />}
      </div>

      <div className="text-sm py-1">{formatPayload(payload, type)}</div>

      {expanded && (
        <div className="mt-2 pt-2 border-t border-gray-100">
          <div className="text-xs text-gray-500 mb-1">Event ID: {id}</div>
          <pre className="text-xs bg-gray-50 p-2 rounded overflow-auto max-h-48">
            {JSON.stringify(payload, null, 2)}
          </pre>
        </div>
      )}
    </div>
  )
}

export default EventCard
