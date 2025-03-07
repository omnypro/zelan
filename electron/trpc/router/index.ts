/**
 * This file now re-exports the appRouter from the server folder
 * to maintain compatibility with existing imports
 */
export { appRouter } from '../server/router';
export type { AppRouter } from '../server/router';