import { EventBus } from '@s/core/bus'
import { TestAdapter, TestAdapterOptions } from './TestAdapter'

/**
 * Create a new test adapter
 */
export function createTestAdapter(
  id: string, 
  name: string, 
  options: Partial<TestAdapterOptions>, 
  eventBus: EventBus
): TestAdapter {
  return new TestAdapter(id, name, options, eventBus)
}

export * from './TestAdapter'