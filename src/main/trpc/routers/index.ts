import { router } from '@s/trpc'
import { adaptersRouter } from './adapters-router'
import { authRouter } from './auth-router'
import { configRouter } from './config-router'
import { eventsRouter } from './events-router'
import { reconnectionRouter } from './reconnection-router'
import { websocketRouter } from './websocket-router'

// Create the root router
export const appRouter = router({
  adapters: adaptersRouter,
  auth: authRouter,
  config: configRouter,
  events: eventsRouter,
  reconnection: reconnectionRouter,
  websocket: websocketRouter
})

// Export type
export type AppRouter = typeof appRouter