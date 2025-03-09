import { EventBus } from '@s/core/bus'
import { TwitchAdapter, TwitchAdapterOptions } from './TwitchAdapter'

/**
 * Create a new Twitch adapter
 */
export function createTwitchAdapter(
  id: string, 
  name: string, 
  options: Partial<TwitchAdapterOptions>, 
  eventBus: EventBus
): TwitchAdapter {
  return new TwitchAdapter(id, name, options, eventBus)
}

export * from './TwitchAdapter'