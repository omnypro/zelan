import Store from 'electron-store';
import { z } from 'zod';

/**
 * Base schema for all adapter settings
 */
export const BaseAdapterSettingsSchema = z.object({
  enabled: z.boolean().default(false),
  autoConnect: z.boolean().default(false),
  name: z.string(),
  id: z.string(),
});

export type BaseAdapterSettings = z.infer<typeof BaseAdapterSettingsSchema>;

/**
 * Schema for Twitch adapter settings
 */
export const TwitchAdapterSettingsSchema = BaseAdapterSettingsSchema.extend({
  channel: z.string().optional(),
  clientId: z.string().optional(),
  redirectUri: z.string().optional(),
  scopes: z.array(z.string()).default([]),
});

export type TwitchAdapterSettings = z.infer<typeof TwitchAdapterSettingsSchema>;

/**
 * Schema for OBS adapter settings
 */
export const ObsAdapterSettingsSchema = BaseAdapterSettingsSchema.extend({
  host: z.string().default('localhost'),
  port: z.number().int().positive().default(4455),
  password: z.string().optional(),
  secure: z.boolean().default(false),
});

export type ObsAdapterSettings = z.infer<typeof ObsAdapterSettingsSchema>;

/**
 * Schema for test adapter settings
 */
export const TestAdapterSettingsSchema = BaseAdapterSettingsSchema.extend({
  interval: z.number().int().positive().default(2000),
  generateErrors: z.boolean().default(false),
});

export type TestAdapterSettings = z.infer<typeof TestAdapterSettingsSchema>;

/**
 * Union type for all adapter settings
 */
export type AdapterSettings = 
  | TwitchAdapterSettings
  | ObsAdapterSettings
  | TestAdapterSettings;

/**
 * Schema for adapter settings storage
 */
export const AdapterSettingsStoreSchema = z.record(z.string(), z.union([
  TwitchAdapterSettingsSchema,
  ObsAdapterSettingsSchema,
  TestAdapterSettingsSchema,
]));

/**
 * Manager for adapter-specific settings
 */
export class AdapterSettingsManager {
  private static instance: AdapterSettingsManager;
  private store: Store<z.infer<typeof AdapterSettingsStoreSchema>>;
  
  private constructor() {
    this.store = new Store({
      name: 'adapter-settings',
      // Default settings for built-in adapters
      defaults: {
        'twitch-adapter': {
          enabled: false,
          autoConnect: false,
          name: 'Twitch',
          id: 'twitch-adapter',
          channel: '',
          clientId: '',
          redirectUri: 'http://localhost',
          scopes: ['channel:read:subscriptions', 'channel:read:redemptions'],
        },
        'obs-adapter': {
          enabled: false,
          autoConnect: false,
          name: 'OBS Studio',
          id: 'obs-adapter',
          host: 'localhost',
          port: 4455,
          secure: false,
        },
        'test-adapter': {
          enabled: true,
          autoConnect: false,
          name: 'Test Adapter',
          id: 'test-adapter',
          interval: 2000,
          generateErrors: false,
        },
      },
    }) as Store<z.infer<typeof AdapterSettingsStoreSchema>>;
  }
  
  /**
   * Get singleton instance of AdapterSettingsManager
   */
  public static getInstance(): AdapterSettingsManager {
    if (!AdapterSettingsManager.instance) {
      AdapterSettingsManager.instance = new AdapterSettingsManager();
    }
    return AdapterSettingsManager.instance;
  }
  
  /**
   * Get settings for all adapters
   */
  public getAllSettings(): Record<string, AdapterSettings> {
    return this.store.store;
  }
  
  /**
   * Get settings for a specific adapter
   */
  public getSettings<T extends AdapterSettings>(adapterId: string): T | null {
    try {
      const settings = this.store.get(adapterId);
      return settings as T || null;
    } catch (error) {
      console.error(`Error retrieving settings for adapter ${adapterId}:`, error);
      return null;
    }
  }
  
  /**
   * Update settings for a specific adapter
   */
  public updateSettings<T extends AdapterSettings>(adapterId: string, settings: Partial<T>): void {
    try {
      const existingSettings = this.getSettings<T>(adapterId);
      if (!existingSettings) {
        throw new Error(`Adapter ${adapterId} not found`);
      }
      
      // Merge existing settings with new settings
      const updatedSettings = {
        ...existingSettings,
        ...settings,
      };
      
      // Validate settings based on adapter type
      let validatedSettings: T;
      
      if (adapterId === 'twitch-adapter') {
        validatedSettings = TwitchAdapterSettingsSchema.parse(updatedSettings) as T;
      } else if (adapterId === 'obs-adapter') {
        validatedSettings = ObsAdapterSettingsSchema.parse(updatedSettings) as T;
      } else if (adapterId === 'test-adapter') {
        validatedSettings = TestAdapterSettingsSchema.parse(updatedSettings) as T;
      } else {
        validatedSettings = BaseAdapterSettingsSchema.parse(updatedSettings) as T;
      }
      
      this.store.set(adapterId, validatedSettings);
    } catch (error) {
      console.error(`Error updating settings for adapter ${adapterId}:`, error);
      throw new Error(`Failed to update adapter settings: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Register a new adapter with default settings
   */
  public registerAdapter(
    adapterId: string, 
    name: string, 
    defaultSettings: Omit<BaseAdapterSettings, 'id' | 'name'>
  ): void {
    try {
      // Check if adapter already exists
      if (this.store.has(adapterId)) {
        return; // Adapter already registered
      }
      
      // Create base settings
      const settings: BaseAdapterSettings = {
        id: adapterId,
        name,
        ...defaultSettings,
      };
      
      this.store.set(adapterId, settings);
    } catch (error) {
      console.error(`Error registering adapter ${adapterId}:`, error);
      throw new Error(`Failed to register adapter: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Delete settings for a specific adapter
   */
  public deleteSettings(adapterId: string): void {
    try {
      this.store.delete(adapterId);
    } catch (error) {
      console.error(`Error deleting settings for adapter ${adapterId}:`, error);
    }
  }
  
  /**
   * Enable or disable an adapter
   */
  public setAdapterEnabled(adapterId: string, enabled: boolean): void {
    try {
      const settings = this.getSettings(adapterId);
      if (!settings) {
        throw new Error(`Adapter ${adapterId} not found`);
      }
      
      this.updateSettings(adapterId, { enabled });
    } catch (error) {
      console.error(`Error updating enabled state for adapter ${adapterId}:`, error);
      throw new Error(`Failed to update adapter state: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Set auto-connect for an adapter
   */
  public setAdapterAutoConnect(adapterId: string, autoConnect: boolean): void {
    try {
      const settings = this.getSettings(adapterId);
      if (!settings) {
        throw new Error(`Adapter ${adapterId} not found`);
      }
      
      this.updateSettings(adapterId, { autoConnect });
    } catch (error) {
      console.error(`Error updating auto-connect for adapter ${adapterId}:`, error);
      throw new Error(`Failed to update adapter auto-connect: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Get all enabled adapters
   */
  public getEnabledAdapters(): string[] {
    const allSettings = this.getAllSettings();
    return Object.entries(allSettings)
      .filter(([_, settings]) => settings.enabled)
      .map(([id]) => id);
  }
  
  /**
   * Get all auto-connect adapters
   */
  public getAutoConnectAdapters(): string[] {
    const allSettings = this.getAllSettings();
    return Object.entries(allSettings)
      .filter(([_, settings]) => settings.enabled && settings.autoConnect)
      .map(([id]) => id);
  }
}