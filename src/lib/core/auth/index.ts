// This file is now deprecated - import from @shared/types instead
import { TokenManager } from './tokenManager';
import { 
  AuthState, 
  AuthProvider, 
  AuthEventSchema, 
  AuthEvent,
  TokenSchema,
  Token
} from '@shared/types';

// Re-export for backward compatibility
export { TokenManager };
export { 
  AuthState, 
  AuthProvider, 
  AuthEventSchema,
  TokenSchema
};
export type { 
  AuthEvent,
  Token
};