import { useCallback, useEffect, useState } from 'react';
import { AuthService, AuthState, Token } from '../core/auth';
import { useObservable } from './useObservable';

/**
 * Hook for interacting with the AuthService
 * Provides authentication state and functions for a service
 */
export function useAuth(serviceId: string) {
  const authService = AuthService.getInstance();
  const [token, setToken] = useState<Token | null>(null);
  
  // Get authentication state as an observable
  const authState = useObservable(
    authService.authState$(serviceId),
    authService.getAuthState(serviceId)
  );
  
  // Convenience state getters
  const isAuthenticated = authState === AuthState.AUTHENTICATED;
  const isAuthenticating = authState === AuthState.AUTHENTICATING;
  const isRefreshing = authState === AuthState.REFRESHING;
  const hasError = authState === AuthState.ERROR;
  
  // Function to start authentication
  const authenticate = useCallback(async () => {
    await authService.authenticate(serviceId);
  }, [serviceId]);
  
  // Function to logout
  const logout = useCallback(async () => {
    await authService.logout(serviceId);
    setToken(null);
  }, [serviceId]);
  
  // Get the current token
  const getToken = useCallback(async (): Promise<Token | null> => {
    const currentToken = await authService.getToken(serviceId);
    setToken(currentToken);
    return currentToken;
  }, [serviceId]);
  
  // Load token on mount and when auth state changes
  useEffect(() => {
    if (isAuthenticated) {
      getToken().catch(console.error);
    }
  }, [isAuthenticated, getToken]);
  
  return {
    authState,
    token,
    isAuthenticated,
    isAuthenticating,
    isRefreshing,
    hasError,
    authenticate,
    logout,
    getToken,
  };
}