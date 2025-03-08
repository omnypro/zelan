import { BaseAdapterFactory } from '../../../shared/adapters/base';
import { EventBus } from '../../../shared/core/bus';
import { ObsAdapter, ObsAdapterOptions } from './ObsAdapter';

/**
 * Factory for creating OBS adapters
 */
export class ObsAdapterFactory extends BaseAdapterFactory<ObsAdapter> {
  constructor() {
    super('obs');
  }
  
  /**
   * Create a new OBS adapter
   */
  create(
    id: string,
    name: string,
    options: Partial<ObsAdapterOptions>,
    eventBus: EventBus
  ): ObsAdapter {
    return new ObsAdapter(id, name, options, eventBus);
  }
}