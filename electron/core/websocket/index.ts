import { z } from 'zod'

export const WebSocketConfigSchema = z.object({
  port: z.number().int().positive().default(8080),
  pingInterval: z.number().int().positive().default(30000),
  path: z.string().default('/events')
})

export type WebSocketConfig = z.infer<typeof WebSocketConfigSchema>

export const WebSocketStatusSchema = z.object({
  isRunning: z.boolean(),
  clientCount: z.number()
})

export type WebSocketStatus = z.infer<typeof WebSocketStatusSchema>

export { WebSocketServer } from './websocketServer'