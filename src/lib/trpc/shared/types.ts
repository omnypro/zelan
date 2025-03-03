import { z } from 'zod'

/**
 * Adapter related types
 */
export const AdapterStatusSchema = z.object({
  status: z.string(),
  isConnected: z.boolean(),
  config: z.record(z.any()).optional()
})
export type AdapterStatus = z.infer<typeof AdapterStatusSchema>

export const OperationResultSchema = z.object({
  success: z.boolean(),
  error: z.string().optional()
})
export type OperationResult = z.infer<typeof OperationResultSchema>

/**
 * WebSocket related types
 */
export const WebSocketStatusSchema = z.object({
  isRunning: z.boolean(),
  clientCount: z.number()
})
export type WebSocketStatus = z.infer<typeof WebSocketStatusSchema>

export const WebSocketConfigSchema = z.object({
  port: z.number().default(8080),
  pingInterval: z.number().default(30000),
  path: z.string().default('/events')
})
export type WebSocketConfig = z.infer<typeof WebSocketConfigSchema>

/**
 * Auth related types
 */
export const AuthStateSchema = z.object({
  state: z.string(),
  isAuthenticated: z.boolean()
})
export type AuthState = z.infer<typeof AuthStateSchema>

/**
 * Event related types
 */
export const BaseEventSchema = z.object({
  id: z.string(),
  type: z.string(),
  source: z.string(),
  timestamp: z.number()
})
export type BaseEvent = z.infer<typeof BaseEventSchema>

export const EventSchema = BaseEventSchema.extend({
  data: z.record(z.any()).optional()
})
export type Event = z.infer<typeof EventSchema>

export const EventsResponseSchema = z.object({
  events: z.array(EventSchema)
})
export type EventsResponse = z.infer<typeof EventsResponseSchema>

export const EventFilterSchema = z.object({
  type: z.string().optional(),
  source: z.string().optional(),
  count: z.number().default(10)
})
export type EventFilter = z.infer<typeof EventFilterSchema>
