import { RendererEventBus, IPC_CHANNELS } from './RendererEventBus'

/**
 * Singleton instance of the RendererEventBus
 */
export const rendererEventBus = new RendererEventBus()

/**
 * Export the IPC channels
 */
export { IPC_CHANNELS }

/**
 * Export the RendererEventBus type
 */
export type { RendererEventBus }
