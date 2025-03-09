import { BaseAdapterFactory } from '@s/adapters/base/BaseAdapterFactory'
import { EventBus } from '@s/core/bus'
import { TwitchAdapter, TwitchAdapterOptions } from './TwitchAdapter'

/**
 * Factory for creating Twitch adapters
 */
export class TwitchAdapterFactory extends BaseAdapterFactory<TwitchAdapter> {
  private eventBus: EventBus;
  
  constructor(eventBus: EventBus) {
    super('twitch')
    this.eventBus = eventBus
  }

  create(
    id: string,
    name: string,
    options: Partial<TwitchAdapterOptions>,
    eventBus?: EventBus
  ): TwitchAdapter {
    // Use the provided eventBus or the one from constructor
    const bus = eventBus || this.eventBus
    return new TwitchAdapter(id, name, options, bus, true)
  }
}

/**
 * Singleton instance of TwitchAdapterFactory
 */
let twitchAdapterFactoryInstance: TwitchAdapterFactory | null = null

/**
 * Get the Twitch adapter factory instance
 */
export function getTwitchAdapterFactory(eventBus: EventBus): TwitchAdapterFactory {
  if (!twitchAdapterFactoryInstance) {
    twitchAdapterFactoryInstance = new TwitchAdapterFactory(eventBus)
  }
  return twitchAdapterFactoryInstance
}