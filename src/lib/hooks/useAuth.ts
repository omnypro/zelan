import { useCallback, useEffect, useState } from 'react';
import { AuthService, AuthState, Token } from '../core/auth';
import { useObservable } from './useObservable';

/**
 * Hook for interacting with the AuthService
 * Provides authentication state and functions for a service
 */
export function useAuth(serviceId: string) {
  const authService = AuthService.getInstance();
  
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
  }, [serviceId]);
  
  // Get the current token
  const getToken = useCallback((): Token | null => {
    return authService.getToken(serviceId);
  }, [serviceId]);
  
  return {
    authState,
    isAuthenticated,
    isAuthenticating,
    isRefreshing,
    hasError,
    authenticate,
    logout,
    getToken,
  };
}