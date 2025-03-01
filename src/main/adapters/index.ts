import { AdapterFactory } from '@shared/adapters'
import { TestAdapterFactory } from './test'

/**
 * All available adapter factories
 */
export const adapterFactories: AdapterFactory[] = [new TestAdapterFactory()]

// Export adapters
export * from './test'
