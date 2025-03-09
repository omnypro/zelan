import { EventBus } from '@s/core/bus'
import { ObsAdapter, ObsAdapterOptions } from './ObsAdapter'

/**
 * Create a new OBS adapter
 */
export function createObsAdapter(
  id: string, 
  name: string, 
  options: Partial<ObsAdapterOptions>, 
  eventBus: EventBus
): ObsAdapter {
  return new ObsAdapter(id, name, options, eventBus)
}

export * from './ObsAdapter'