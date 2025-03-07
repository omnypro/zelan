// Re-export from electron implementation
export type { Token } from '~/core/auth';
export { 
  TokenSchema,
  AuthState,
  AuthProvider,
  AuthService,
  AuthEvent,
  AuthEventSchema
} from '~/core/auth';

// Still need the tokenManager from renderer for UI
export * from './tokenManager';