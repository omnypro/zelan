import { z } from 'zod';

/**
 * WebSocket client connection info
 */
export interface WebSocketClient {
  id: string;
  ip: string;
  connectedAt: number;
  lastPingAt: number;
  userAgent?: string;
}

/**
 * WebSocket server status
 */
export interface WebSocketServerStatus {
  running: boolean;
  port: number;
  clientCount: number;
  uptime: number;
}

/**
 * WebSocket message schema
 */
export const WebSocketMessageSchema = z.object({
  type: z.string(),
  payload: z.any(),
  timestamp: z.number().optional(),
});

export type WebSocketMessage = z.infer<typeof WebSocketMessageSchema>;