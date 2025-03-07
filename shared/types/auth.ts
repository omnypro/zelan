import { z } from 'zod';
import { BaseEventSchema } from './events';
import { Token } from './tokens';

/**
 * Authentication state enum
 */
export enum AuthState {
  UNKNOWN = 'unknown',
  UNAUTHENTICATED = 'unauthenticated',
  AUTHENTICATING = 'authenticating',
  AUTHENTICATED = 'authenticated',
  REFRESHING = 'refreshing',
  ERROR = 'error',
}

/**
 * Auth provider interface that all service-specific auth providers must implement
 */
export interface AuthProvider {
  readonly serviceId: string;
  startAuth(): Promise<void>;
  refreshToken(refreshToken: string): Promise<Token>;
  revokeToken(token: Token): Promise<void>;
}

/**
 * Auth event schema built on top of the base event
 */
export const AuthEventSchema = BaseEventSchema.extend({
  serviceId: z.string(),
  state: z.nativeEnum(AuthState),
  error: z.string().optional(),
});

export type AuthEvent = z.infer<typeof AuthEventSchema>;