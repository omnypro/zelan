// Export auth components for electron main process
export { TokenManager } from './tokenManager';
export { AuthService } from './authService';

// Export Auth State enum for convenience
export enum AuthState {
  UNAUTHENTICATED = 'unauthenticated',
  AUTHENTICATING = 'authenticating',
  AUTHENTICATED = 'authenticated',
  ERROR = 'error',
}