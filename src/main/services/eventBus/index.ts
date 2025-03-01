import { MainEventBus, IPC_CHANNELS } from './MainEventBus';

/**
 * Singleton instance of the MainEventBus
 */
export const mainEventBus = new MainEventBus();

/**
 * Export the IPC channels
 */
export { IPC_CHANNELS };

/**
 * Export the MainEventBus type
 */
export type { MainEventBus };