import { useMemo } from 'react'
import { EventCategory } from '@s/types/events'

interface TimelineEvent {
  id: string
  timestamp: number
  category?: EventCategory
}

interface EventTimelineProps {
  events: TimelineEvent[]
  timeRange: number // in minutes
  height?: number
  onTimeClick?: (timestamp: number) => void
}

interface TimeSegment {
  timestamp: number
  count: number
  categories: Record<string, number>
}

/**
 * Visualizes events on a timeline showing frequency by time segment
 */
export function EventTimeline({ events, timeRange, height = 100, onTimeClick }: EventTimelineProps) {
  // Calculate time segments and event distribution
  const { timeSegments, maxCount, startTime, endTime } = useMemo(() => {
    if (!events.length) {
      return { timeSegments: [], maxCount: 0, startTime: Date.now(), endTime: Date.now() }
    }

    const now = Date.now()
    const startTimeMs = now - timeRange * 60 * 1000
    const segmentCount = Math.min(50, timeRange) // Max 50 segments
    const segmentSize = (timeRange * 60 * 1000) / segmentCount

    // Initialize segments
    const segments: TimeSegment[] = []
    for (let i = 0; i < segmentCount; i++) {
      segments.push({
        timestamp: startTimeMs + i * segmentSize,
        count: 0,
        categories: {}
      })
    }

    // Count events in each segment
    events.forEach((event) => {
      if (event.timestamp < startTimeMs) return

      const segmentIndex = Math.floor((event.timestamp - startTimeMs) / segmentSize)
      if (segmentIndex >= 0 && segmentIndex < segmentCount) {
        const segment = segments[segmentIndex]
        segment.count++

        const category = event.category || 'unknown'
        segment.categories[category] = (segment.categories[category] || 0) + 1
      }
    })

    // Find max count for scaling
    const maxSegmentCount = Math.max(1, ...segments.map((s) => s.count))

    return {
      timeSegments: segments,
      maxCount: maxSegmentCount,
      startTime: startTimeMs,
      endTime: now
    }
  }, [events, timeRange])

  // Category colors for the timeline
  const categoryColors: Record<string, string> = {
    [EventCategory.SYSTEM]: 'bg-blue-400',
    [EventCategory.ADAPTER]: 'bg-purple-400',
    [EventCategory.SERVICE]: 'bg-yellow-400',
    [EventCategory.TWITCH]: 'bg-violet-400',
    [EventCategory.OBS]: 'bg-pink-400',
    [EventCategory.USER]: 'bg-green-400',
    unknown: 'bg-gray-400'
  }

  // Format timestamp for display
  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }

  // Handle clicking on a time segment
  const handleSegmentClick = (timestamp: number) => {
    if (onTimeClick) {
      onTimeClick(timestamp)
    }
  }

  return (
    <div className="bg-white rounded-md shadow-sm border p-3 mb-4">
      <div className="flex justify-between items-center mb-2">
        <h3 className="text-md font-semibold">Event Timeline</h3>
        <div className="text-xs text-gray-500">
          {formatTime(startTime)} - {formatTime(endTime)}
        </div>
      </div>

      {/* Timeline Visualization */}
      <div className="relative w-full bg-gray-50 border rounded" style={{ height: `${height}px` }}>
        {timeSegments.length === 0 ? (
          <div className="absolute inset-0 flex items-center justify-center text-sm text-gray-400">
            No events in selected time range
          </div>
        ) : (
          <>
            {/* Time segments */}
            <div className="flex h-full w-full">
              {timeSegments.map((segment, index) => (
                <div
                  key={index}
                  className="flex-1 h-full relative hover:bg-gray-100 cursor-pointer group"
                  onClick={() => handleSegmentClick(segment.timestamp)}
                >
                  {/* Stacked bars for categories */}
                  <div className="absolute bottom-0 left-0 right-0 flex flex-col-reverse">
                    {Object.entries(segment.categories).map(([category, count]) => {
                      const height = (count / maxCount) * 100
                      return (
                        <div
                          key={category}
                          className={`w-full ${categoryColors[category] || 'bg-gray-400'}`}
                          style={{ height: `${height}%` }}
                          title={`${count} ${category} events`}
                        />
                      )
                    })}
                  </div>

                  {/* Hover tooltip */}
                  <div className="absolute bottom-full left-1/2 transform -translate-x-1/2 mb-1 hidden group-hover:block z-10">
                    <div className="bg-gray-800 text-white text-xs rounded py-1 px-2 whitespace-nowrap">
                      <div>{formatTime(segment.timestamp)}</div>
                      <div>{segment.count} events</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {/* Time labels */}
            <div className="absolute bottom-0 left-0 right-0 flex justify-between text-xs text-gray-500 pb-1 px-2">
              <span>{formatTime(startTime)}</span>
              <span>{formatTime(startTime + (endTime - startTime) / 2)}</span>
              <span>{formatTime(endTime)}</span>
            </div>
          </>
        )}
      </div>

      {/* Legend */}
      <div className="flex flex-wrap gap-3 mt-2">
        {Object.entries(categoryColors).map(
          ([category, color]) =>
            category !== 'unknown' && (
              <div key={category} className="flex items-center gap-1">
                <div className={`w-3 h-3 ${color} rounded-sm`} />
                <span className="text-xs">{category}</span>
              </div>
            )
        )}
      </div>
    </div>
  )
}

export default EventTimeline
