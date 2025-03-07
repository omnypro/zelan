import { z } from 'zod';

/**
 * App configuration schema
 */
export const AppConfigSchema = z.object({
  firstRun: z.boolean().default(true),
  theme: z.enum(['light', 'dark', 'system']).default('system'),
  logLevel: z.enum(['error', 'warn', 'info', 'debug']).default('info'),
  startOnLogin: z.boolean().default(false),
  minimizeToTray: z.boolean().default(true),
});

export type AppConfig = z.infer<typeof AppConfigSchema>;

/**
 * WebSocket server configuration schema
 */
export const WebSocketConfigSchema = z.object({
  enabled: z.boolean().default(true),
  port: z.number().default(30000),
  path: z.string().default('/ws'),
  pingInterval: z.number().default(30000),
  cors: z.object({
    enabled: z.boolean().default(true),
    origins: z.array(z.string()).default(['*']),
  }),
});

export type WebSocketConfig = z.infer<typeof WebSocketConfigSchema>;

/**
 * Event system configuration schema
 */
export const EventConfigSchema = z.object({
  maxCachedEvents: z.number().default(1000),
  logToConsole: z.boolean().default(false),
});

export type EventConfig = z.infer<typeof EventConfigSchema>;

/**
 * Full application configuration schema
 */
export const ConfigSchema = z.object({
  app: AppConfigSchema,
  websocket: WebSocketConfigSchema,
  events: EventConfigSchema,
});

export type Config = z.infer<typeof ConfigSchema>;