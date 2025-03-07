// This file is now deprecated - import from @shared/types instead
import { TokenManager } from './tokenManager';
import { AuthService } from './authService';
import { 
  AuthState, 
  AuthProvider, 
  AuthEventSchema, 
  TokenSchema,
} from '@shared/types';
import type { 
  AuthEvent,
  Token 
} from '@shared/types';

// Re-export for backward compatibility
export { TokenManager, AuthService };
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