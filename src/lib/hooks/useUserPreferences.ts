import { useState, useEffect, useCallback, useRef } from 'react';
import { useTrpc } from './useTrpc';

// Preferences types
interface UiPreferences {
  theme: 'light' | 'dark' | 'system';
  sidebarCollapsed: boolean;
  fontSize?: 'small' | 'medium' | 'large';
}

interface NotificationPreferences {
  enabled: boolean;
  sound?: boolean;
  desktop?: boolean;
}

interface UserPreferences {
  ui: UiPreferences;
  notifications: NotificationPreferences;
}

// Default preferences
const defaultPreferences: UserPreferences = {
  ui: {
    theme: 'system',
    sidebarCollapsed: false,
    fontSize: 'medium'
  },
  notifications: {
    enabled: true,
    sound: true,
    desktop: true
  }
};

/**
 * Hook for managing user preferences
 * Uses tRPC to communicate with the main process
 */
export function useUserPreferences() {
  const { client } = useTrpc();
  const [ui, setUi] = useState<UiPreferences>(defaultPreferences.ui);
  const [notifications, setNotifications] = useState<NotificationPreferences>(defaultPreferences.notifications);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const isMounted = useRef(true);

  // Fetch preferences from backend
  const fetchPreferences = useCallback(async () => {
    if (!client) return null;
    
    setIsLoading(true);
    setError(null);
    
    try {
      const response = await client.config.getUserData.query();
      
      if (isMounted.current) {
        if (response.success) {
          const data = response.data as Record<string, unknown>;
          const userPrefs: UserPreferences = { ...defaultPreferences };
          
          // Extract UI preferences
          if (data.ui && typeof data.ui === 'object') {
            const ui = data.ui as Record<string, unknown>;
            userPrefs.ui = {
              ...userPrefs.ui,
              theme: (ui.theme as 'light' | 'dark' | 'system') || userPrefs.ui.theme,
              sidebarCollapsed: Boolean(ui.sidebarCollapsed),
              fontSize: (ui.fontSize as 'small' | 'medium' | 'large') || userPrefs.ui.fontSize
            };
          }
          
          // Extract notification preferences
          if (data.notifications && typeof data.notifications === 'object') {
            const notifications = data.notifications as Record<string, unknown>;
            userPrefs.notifications = {
              ...userPrefs.notifications,
              enabled: Boolean(notifications.enabled),
              sound: Boolean(notifications.sound),
              desktop: Boolean(notifications.desktop)
            };
          }
          
          setUi(userPrefs.ui);
          setNotifications(userPrefs.notifications);
          
          return userPrefs;
        } else {
          setError(response.error || 'Failed to fetch preferences');
          return null;
        }
      }
    } catch (error) {
      if (isMounted.current) {
        console.error('Error fetching preferences:', error);
        setError(error instanceof Error ? error.message : 'Failed to fetch preferences');
      }
      return null;
    } finally {
      if (isMounted.current) {
        setIsLoading(false);
      }
    }
  }, [client]);

  // Update UI preferences
  const updateUiPreferences = useCallback(async (newPreferences: Partial<UiPreferences>) => {
    if (!client) return false;
    
    // Optimistic update
    const updatedUi = { ...ui, ...newPreferences };
    setUi(updatedUi);
    
    try {
      const result = await client.config.updateUserData.mutate({
        ui: newPreferences
      });
      
      if (!result.success) {
        fetchPreferences(); // Revert on error
      }
      
      return result.success;
    } catch (error) {
      console.error('Error updating UI preferences:', error);
      fetchPreferences(); // Revert on error
      return false;
    }
  }, [client, ui, fetchPreferences]);

  // Update notification preferences
  const updateNotificationPreferences = useCallback(async (newPreferences: Partial<NotificationPreferences>) => {
    if (!client) return false;
    
    // Optimistic update
    const updatedNotifications = { ...notifications, ...newPreferences };
    setNotifications(updatedNotifications);
    
    try {
      const result = await client.config.updateUserData.mutate({
        notifications: newPreferences
      });
      
      if (!result.success) {
        fetchPreferences(); // Revert on error
      }
      
      return result.success;
    } catch (error) {
      console.error('Error updating notification preferences:', error);
      fetchPreferences(); // Revert on error
      return false;
    }
  }, [client, notifications, fetchPreferences]);

  // Helper methods
  const setTheme = useCallback((theme: 'light' | 'dark' | 'system') => {
    return updateUiPreferences({ theme });
  }, [updateUiPreferences]);

  const toggleSidebar = useCallback(() => {
    return updateUiPreferences({ sidebarCollapsed: !ui.sidebarCollapsed });
  }, [ui.sidebarCollapsed, updateUiPreferences]);

  const toggleNotifications = useCallback(() => {
    return updateNotificationPreferences({ enabled: !notifications.enabled });
  }, [notifications.enabled, updateNotificationPreferences]);

  // Fetch on mount
  useEffect(() => {
    isMounted.current = true;
    
    if (client) {
      fetchPreferences();
    }
    
    return () => {
      isMounted.current = false;
    };
  }, [client, fetchPreferences]);

  return {
    ui,
    notifications,
    isLoading,
    error,
    updateUiPreferences,
    updateNotificationPreferences,
    setTheme,
    toggleSidebar,
    toggleNotifications
  };
}