import { useState } from 'react'
import { EventCategory } from '@s/types/events'
import { EventFilterCriteria } from '@s/utils/filters/event-filter'

interface EventFilterBarProps {
  onFilterChange: (criteria: EventFilterCriteria) => void
  onTimeRangeChange: (minutes: number) => void
  categories?: EventCategory[]
  sources?: Array<{ id: string; type: string }>
  types?: string[]
}

/**
 * Filter toolbar for the event viewer
 */
export function EventFilterBar({
  onFilterChange,
  onTimeRangeChange,
  categories = Object.values(EventCategory),
  sources = [],
  types = []
}: EventFilterBarProps) {
  const [selectedCategory, setSelectedCategory] = useState<EventCategory | ''>('')
  const [selectedType, setSelectedType] = useState<string>('')
  const [selectedSource, setSelectedSource] = useState<string>('')
  const [timeRange, setTimeRange] = useState<number>(60) // minutes
  const [showAdvanced, setShowAdvanced] = useState(false)
  
  // Apply filters when they change
  const applyFilters = () => {
    const criteria: EventFilterCriteria = {}
    
    if (selectedCategory) {
      criteria.category = selectedCategory as EventCategory
    }
    
    if (selectedType) {
      criteria.type = selectedType
    }
    
    if (selectedSource) {
      const [sourceId, sourceType] = selectedSource.split('|')
      if (sourceId) criteria.sourceId = sourceId
      if (sourceType) criteria.sourceType = sourceType
    }
    
    onFilterChange(criteria)
    onTimeRangeChange(timeRange)
  }
  
  // Reset all filters
  const resetFilters = () => {
    setSelectedCategory('')
    setSelectedType('')
    setSelectedSource('')
    setTimeRange(60)
    
    onFilterChange({})
    onTimeRangeChange(60)
  }
  
  // Handle category change
  const handleCategoryChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedCategory(e.target.value as EventCategory | '')
    setTimeout(applyFilters, 0)
  }
  
  // Handle type change
  const handleTypeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedType(e.target.value)
    setTimeout(applyFilters, 0)
  }
  
  // Handle source change
  const handleSourceChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedSource(e.target.value)
    setTimeout(applyFilters, 0)
  }
  
  // Handle time range change
  const handleTimeRangeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const minutes = parseInt(e.target.value, 10)
    setTimeRange(minutes)
    onTimeRangeChange(minutes)
  }

  return (
    <div className="p-3 bg-white border rounded-md shadow-sm mb-4">
      <div className="flex flex-wrap gap-3 items-center">
        {/* Category Filter */}
        <div className="flex-1 min-w-[150px]">
          <label className="block text-xs font-medium text-gray-500 mb-1">Category</label>
          <select
            value={selectedCategory}
            onChange={handleCategoryChange}
            className="w-full p-1.5 text-sm border rounded"
          >
            <option value="">All Categories</option>
            {categories.map((cat) => (
              <option key={cat} value={cat}>
                {cat}
              </option>
            ))}
          </select>
        </div>
        
        {/* Type Filter */}
        <div className="flex-1 min-w-[150px]">
          <label className="block text-xs font-medium text-gray-500 mb-1">Type</label>
          <select
            value={selectedType}
            onChange={handleTypeChange}
            className="w-full p-1.5 text-sm border rounded"
          >
            <option value="">All Types</option>
            {types.map((type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ))}
          </select>
        </div>
        
        {/* Source Filter */}
        <div className="flex-1 min-w-[150px]">
          <label className="block text-xs font-medium text-gray-500 mb-1">Source</label>
          <select
            value={selectedSource}
            onChange={handleSourceChange}
            className="w-full p-1.5 text-sm border rounded"
          >
            <option value="">All Sources</option>
            {sources.map((source) => (
              <option key={`${source.id}|${source.type}`} value={`${source.id}|${source.type}`}>
                {source.id} ({source.type})
              </option>
            ))}
          </select>
        </div>
        
        {/* Time Range Filter */}
        <div className="flex-1 min-w-[150px]">
          <label className="block text-xs font-medium text-gray-500 mb-1">Time Range</label>
          <select
            value={timeRange}
            onChange={handleTimeRangeChange}
            className="w-full p-1.5 text-sm border rounded"
          >
            <option value="5">Last 5 minutes</option>
            <option value="15">Last 15 minutes</option>
            <option value="30">Last 30 minutes</option>
            <option value="60">Last hour</option>
            <option value="360">Last 6 hours</option>
            <option value="1440">Last 24 hours</option>
            <option value="10080">Last week</option>
          </select>
        </div>
        
        {/* Reset Button */}
        <div className="flex items-end">
          <button 
            onClick={resetFilters}
            className="p-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded"
          >
            Reset Filters
          </button>
        </div>
        
        {/* Advanced Toggle */}
        <div className="flex items-end ml-auto">
          <button 
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="p-1.5 text-sm text-blue-500 hover:text-blue-600"
          >
            {showAdvanced ? 'Hide Advanced' : 'Advanced Options'}
          </button>
        </div>
      </div>
      
      {/* Advanced Options */}
      {showAdvanced && (
        <div className="mt-3 pt-3 border-t">
          <div className="text-xs mb-2">Advanced filtering options</div>
          <div className="flex gap-3">
            {/* Add advanced options here: exact matching, custom predicates, etc. */}
            <div className="text-sm text-gray-500">Coming soon...</div>
          </div>
        </div>
      )}
    </div>
  )
}

export default EventFilterBar