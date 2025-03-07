import Store from 'electron-store';
import { z } from 'zod';

/**
 * Schema for user interface preferences
 */
export const UiPreferencesSchema = z.object({
  dashboardLayout: z.record(z.string(), z.any()).default({}),
  sidebarCollapsed: z.boolean().default(false),
  expandedSections: z.array(z.string()).default([]),
  recentViews: z.array(z.string()).default([]),
  fontSize: z.enum(['small', 'medium', 'large']).default('medium'),
  theme: z.enum(['light', 'dark', 'system']).default('system'),
});

export type UiPreferences = z.infer<typeof UiPreferencesSchema>;

/**
 * Schema for notification preferences
 */
export const NotificationPreferencesSchema = z.object({
  enabled: z.boolean().default(true),
  sound: z.boolean().default(true),
  desktop: z.boolean().default(true),
  mutedEvents: z.array(z.string()).default([]),
  mutedSources: z.array(z.string()).default([]),
});

export type NotificationPreferences = z.infer<typeof NotificationPreferencesSchema>;

/**
 * Schema for all user data
 */
export const UserDataSchema = z.object({
  ui: UiPreferencesSchema.default({}),
  notifications: NotificationPreferencesSchema.default({}),
  customData: z.record(z.string(), z.any()).default({}),
  recentSearches: z.array(z.string()).default([]),
  lastLogin: z.number().optional(),
  sessionCount: z.number().default(0),
});

export type UserData = z.infer<typeof UserDataSchema>;

/**
 * Manager for user data and preferences
 * Handles storing and retrieving user-specific settings
 */
export class UserDataStore {
  private static instance: UserDataStore;
  private store: Store<UserData>;
  
  private constructor() {
    this.store = new Store({
      name: 'user-data',
      defaults: {
        ui: {
          dashboardLayout: {},
          sidebarCollapsed: false,
          expandedSections: [],
          recentViews: [],
          fontSize: 'medium',
          theme: 'system',
        },
        notifications: {
          enabled: true,
          sound: true,
          desktop: true,
          mutedEvents: [],
          mutedSources: [],
        },
        customData: {},
        recentSearches: [],
        sessionCount: 0,
      },
    });
    
    // Increment session count on initialization
    this.incrementSessionCount();
    
    // Update last login timestamp
    this.updateLastLogin();
  }
  
  /**
   * Get singleton instance of UserDataStore
   */
  public static getInstance(): UserDataStore {
    if (!UserDataStore.instance) {
      UserDataStore.instance = new UserDataStore();
    }
    return UserDataStore.instance;
  }
  
  /**
   * Get all user data
   */
  public getData(): UserData {
    return this.store.store;
  }

  /**
   * Update all or part of the user data
   */
  public updateData(data: Partial<UserData>): void {
    // Update each top-level section that exists in the data
    if (data.ui) {
      const currentUi = this.getUiPreferences();
      this.updateUiPreferences({ ...currentUi, ...data.ui });
    }
    
    if (data.notifications) {
      const currentNotifications = this.getNotificationPreferences();
      this.updateNotificationPreferences({ ...currentNotifications, ...data.notifications });
    }
    
    if (data.customData) {
      for (const [key, value] of Object.entries(data.customData)) {
        this.setCustomData(key, value);
      }
    }
  }
  
  /**
   * Get UI preferences
   */
  public getUiPreferences(): UiPreferences {
    return this.store.get('ui');
  }
  
  /**
   * Update UI preferences
   */
  public updateUiPreferences(preferences: Partial<UiPreferences>): void {
    const current = this.getUiPreferences();
    this.store.set('ui', {
      ...current,
      ...preferences,
    });
  }
  
  /**
   * Get notification preferences
   */
  public getNotificationPreferences(): NotificationPreferences {
    return this.store.get('notifications');
  }
  
  /**
   * Update notification preferences
   */
  public updateNotificationPreferences(preferences: Partial<NotificationPreferences>): void {
    const current = this.getNotificationPreferences();
    this.store.set('notifications', {
      ...current,
      ...preferences,
    });
  }
  
  /**
   * Get custom data by key
   */
  public getCustomData<T>(key: string): T | null {
    try {
      const data = this.store.get(`customData.${key}`);
      return data as T || null;
    } catch (error) {
      console.error(`Error retrieving custom data for key ${key}:`, error);
      return null;
    }
  }
  
  /**
   * Set custom data by key
   */
  public setCustomData<T>(key: string, value: T): void {
    try {
      this.store.set(`customData.${key}`, value);
    } catch (error) {
      console.error(`Error setting custom data for key ${key}:`, error);
      throw new Error(`Failed to set custom data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Delete custom data by key
   */
  public deleteCustomData(key: string): void {
    try {
      this.store.delete(`customData.${key}`);
    } catch (error) {
      console.error(`Error deleting custom data for key ${key}:`, error);
    }
  }
  
  /**
   * Add a search term to recent searches
   */
  public addRecentSearch(search: string): void {
    try {
      // Get current recent searches
      const recentSearches = this.store.get('recentSearches');
      
      // Remove the search term if it already exists
      const filteredSearches = recentSearches.filter(s => s !== search);
      
      // Add the search term to the beginning of the array
      filteredSearches.unshift(search);
      
      // Limit to 10 recent searches
      const limitedSearches = filteredSearches.slice(0, 10);
      
      // Save updated recent searches
      this.store.set('recentSearches', limitedSearches);
    } catch (error) {
      console.error('Error adding recent search:', error);
    }
  }
  
  /**
   * Get recent searches
   */
  public getRecentSearches(): string[] {
    return this.store.get('recentSearches');
  }
  
  /**
   * Clear recent searches
   */
  public clearRecentSearches(): void {
    this.store.set('recentSearches', []);
  }
  
  /**
   * Update last login timestamp
   */
  private updateLastLogin(): void {
    this.store.set('lastLogin', Date.now());
  }
  
  /**
   * Get last login timestamp
   */
  public getLastLogin(): number | null {
    return this.store.get('lastLogin') || null;
  }
  
  /**
   * Increment session count
   */
  private incrementSessionCount(): void {
    const currentCount = this.store.get('sessionCount');
    this.store.set('sessionCount', currentCount + 1);
  }
  
  /**
   * Get session count
   */
  public getSessionCount(): number {
    return this.store.get('sessionCount');
  }
  
  /**
   * Save dashboard layout
   */
  public saveDashboardLayout(layout: Record<string, unknown>): void {
    this.updateUiPreferences({ dashboardLayout: layout });
  }
  
  /**
   * Get dashboard layout
   */
  public getDashboardLayout(): Record<string, unknown> {
    return this.getUiPreferences().dashboardLayout;
  }
  
  /**
   * Add a view to recent views
   */
  public addRecentView(viewId: string): void {
    try {
      const ui = this.getUiPreferences();
      const recentViews = ui.recentViews;
      
      // Remove the view if it already exists
      const filteredViews = recentViews.filter(v => v !== viewId);
      
      // Add the view to the beginning of the array
      filteredViews.unshift(viewId);
      
      // Limit to 5 recent views
      const limitedViews = filteredViews.slice(0, 5);
      
      // Save updated recent views
      this.updateUiPreferences({ recentViews: limitedViews });
    } catch (error) {
      console.error('Error adding recent view:', error);
    }
  }
  
  /**
   * Toggle sidebar collapsed state
   */
  public toggleSidebarCollapsed(): boolean {
    const ui = this.getUiPreferences();
    const newState = !ui.sidebarCollapsed;
    this.updateUiPreferences({ sidebarCollapsed: newState });
    return newState;
  }
  
  /**
   * Reset all user data to defaults
   */
  public resetAll(): void {
    this.store.clear();
  }
}