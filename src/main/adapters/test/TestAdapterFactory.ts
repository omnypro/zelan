import { BaseAdapterFactory } from '../../../shared/adapters/base';
import { EventBus } from '../../../shared/core/bus';
import { TestAdapter, TestAdapterOptions } from './TestAdapter';

/**
 * Factory for creating test adapters
 */
export class TestAdapterFactory extends BaseAdapterFactory<TestAdapter> {
  constructor() {
    super('test');
  }
  
  /**
   * Create a new test adapter
   */
  create(
    id: string,
    name: string,
    options: Partial<TestAdapterOptions>,
    eventBus: EventBus
  ): TestAdapter {
    return new TestAdapter(id, name, options, eventBus);
  }
}