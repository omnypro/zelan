import { z } from 'zod';

/**
 * Adapter related types
 */
export const AdapterStatusSchema = z.object({
  status: z.string(),
  isConnected: z.boolean(),
  config: z.record(z.any()).optional(),
});
export type AdapterStatus = z.infer<typeof AdapterStatusSchema>;

export const OperationResultSchema = z.object({
  success: z.boolean(),
  error: z.string().optional(),
});
export type OperationResult = z.infer<typeof OperationResultSchema>;

/**
 * WebSocket related types
 */
export const WebSocketStatusSchema = z.object({
  isRunning: z.boolean(),
  clientCount: z.number(),
});
export type WebSocketStatus = z.infer<typeof WebSocketStatusSchema>;

export const WebSocketConfigSchema = z.object({
  port: z.number().default(8080),
  enableCors: z.boolean().default(true),
});
export type WebSocketConfig = z.infer<typeof WebSocketConfigSchema>;

/**
 * Auth related types
 */
export const AuthStateSchema = z.object({
  state: z.string(),
  isAuthenticated: z.boolean(),
});
export type AuthState = z.infer<typeof AuthStateSchema>;

/**
 * Event related types
 */
export const EventSchema = z.object({
  id: z.string(),
  type: z.string(),
  source: z.string(),
  timestamp: z.number(),
  data: z.record(z.any()),
});
export type Event = z.infer<typeof EventSchema>;

export const EventsResponseSchema = z.object({
  events: z.array(EventSchema),
});
export type EventsResponse = z.infer<typeof EventsResponseSchema>;