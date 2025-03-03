import { useState, useEffect, useCallback } from 'react';
import { useTrpc } from './useTrpc';

// We'll create this procedure in the tRPC router later
interface UiPreferences {
  theme?: 'light' | 'dark' | 'system';
  sidebarCollapsed?: boolean;
  fontSize?: 'small' | 'medium' | 'large';
}

interface NotificationPreferences {
  enabled?: boolean;
  sound?: boolean;
  desktop?: boolean;
}

/**
 * Hook for managing user preferences
 */
export function useUserPreferences() {
  const trpc = useTrpc();
  const [uiPreferences, setUiPreferences] = useState<UiPreferences>({
    theme: 'system',
    sidebarCollapsed: false,
    fontSize: 'medium'
  });
  
  const [notificationPreferences, setNotificationPreferences] = useState<NotificationPreferences>({
    enabled: true,
    sound: true,
    desktop: true
  });
  
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  /**
   * Fetch user preferences from storage
   */
  const fetchPreferences = useCallback(async () => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return null;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      // This will be implemented in the tRPC router later
      // For now, we're just returning dummy data
      
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 100));
      
      const preferences = {
        ui: {
          theme: 'system' as const,
          sidebarCollapsed: false,
          fontSize: 'medium' as const
        },
        notifications: {
          enabled: true,
          sound: true,
          desktop: true
        }
      };
      
      setUiPreferences(preferences.ui);
      setNotificationPreferences(preferences.notifications);
      
      return preferences;
    } catch (error) {
      console.error('Error fetching user preferences:', error);
      const message = error instanceof Error ? error.message : 'Failed to fetch user preferences';
      setError(message);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  /**
   * Update UI preferences
   */
  const updateUiPreferences = useCallback(async (preferences: Partial<UiPreferences>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      // This will be implemented in the tRPC router later
      // For now, we're just updating the local state
      
      setUiPreferences(prev => ({
        ...prev,
        ...preferences
      }));
      
      return true;
    } catch (error) {
      console.error('Error updating UI preferences:', error);
      const message = error instanceof Error ? error.message : 'Failed to update UI preferences';
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  /**
   * Update notification preferences
   */
  const updateNotificationPreferences = useCallback(async (preferences: Partial<NotificationPreferences>) => {
    if (!trpc.client) {
      console.error('TRPC client not available');
      setError('TRPC client not available');
      return false;
    }
    
    setIsLoading(true);
    setError(null);
    
    try {
      // This will be implemented in the tRPC router later
      // For now, we're just updating the local state
      
      setNotificationPreferences(prev => ({
        ...prev,
        ...preferences
      }));
      
      return true;
    } catch (error) {
      console.error('Error updating notification preferences:', error);
      const message = error instanceof Error ? error.message : 'Failed to update notification preferences';
      setError(message);
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [trpc.client]);
  
  /**
   * Set theme preference
   */
  const setTheme = useCallback((theme: 'light' | 'dark' | 'system') => {
    return updateUiPreferences({ theme });
  }, [updateUiPreferences]);
  
  /**
   * Toggle sidebar collapsed state
   */
  const toggleSidebar = useCallback(() => {
    return updateUiPreferences({ sidebarCollapsed: !uiPreferences.sidebarCollapsed });
  }, [updateUiPreferences, uiPreferences.sidebarCollapsed]);
  
  /**
   * Set font size
   */
  const setFontSize = useCallback((fontSize: 'small' | 'medium' | 'large') => {
    return updateUiPreferences({ fontSize });
  }, [updateUiPreferences]);
  
  /**
   * Toggle notifications
   */
  const toggleNotifications = useCallback(() => {
    return updateNotificationPreferences({ enabled: !notificationPreferences.enabled });
  }, [updateNotificationPreferences, notificationPreferences.enabled]);
  
  // Fetch preferences on mount
  useEffect(() => {
    if (trpc.client) {
      fetchPreferences();
    }
  }, [trpc.client, fetchPreferences]);
  
  return {
    ui: uiPreferences,
    notifications: notificationPreferences,
    isLoading,
    error,
    updateUiPreferences,
    updateNotificationPreferences,
    setTheme,
    toggleSidebar,
    setFontSize,
    toggleNotifications
  };
}