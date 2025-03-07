import { app } from 'electron';
import path from 'path';
import fs from 'fs';
import { BehaviorSubject, Observable, Subject } from 'rxjs';
import { distinctUntilChanged, filter, map, shareReplay } from 'rxjs/operators';

/**
 * Adapter configuration interface
 */
export interface AdapterConfig {
  id: string;
  type: string;
  name: string;
  enabled: boolean;
  options: Record<string, any>;
}

/**
 * Authentication token interface
 */
export interface AuthToken {
  service: string;
  accessToken: string;
  refreshToken?: string;
  expiresAt?: number;
  scopes?: string[];
}

/**
 * Application configuration interface
 */
export interface AppConfig {
  adapters: Record<string, AdapterConfig>;
  auth: {
    tokens: Record<string, AuthToken>;
  };
  settings: {
    startOnBoot: boolean;
    minimizeToTray: boolean;
    theme: 'light' | 'dark' | 'system';
  };
}

/**
 * Config change event interface
 */
export interface ConfigChangeEvent {
  key: string;
  value: any;
  previousValue?: any;
}

/**
 * Default configuration object
 */
const DEFAULT_CONFIG: AppConfig = {
  adapters: {},
  auth: {
    tokens: {}
  },
  settings: {
    startOnBoot: false,
    minimizeToTray: true,
    theme: 'system'
  }
};

/**
 * Reactive config store using RxJS
 */
export class ReactiveConfigStore {
  // The main configuration subject
  private configSubject: BehaviorSubject<AppConfig>;
  
  // The change events subject
  private changeSubject: Subject<ConfigChangeEvent>;
  
  // File path for storage
  private filePath: string;
  
  constructor(filename: string = 'zelan-config.json') {
    // Set file path in user data directory
    this.filePath = path.join(app.getPath('userData'), filename);
    
    // Load initial data
    const initialData = this.load();
    
    // Initialize subjects
    this.configSubject = new BehaviorSubject<AppConfig>(initialData);
    this.changeSubject = new Subject<ConfigChangeEvent>();
    
    // Log configuration loaded
    console.log('Configuration loaded from:', this.filePath);
  }
  
  /**
   * Get an observable of the full config
   */
  config$(): Observable<AppConfig> {
    return this.configSubject.asObservable().pipe(
      distinctUntilChanged((prev, curr) => JSON.stringify(prev) === JSON.stringify(curr)),
      shareReplay(1)
    );
  }
  
  /**
   * Get an observable for a specific config path
   */
  select$<T>(path: string): Observable<T> {
    return this.config$().pipe(
      map(config => {
        try {
          return path.split('.').reduce((obj, key) => obj?.[key], config as any) as T;
        } catch (error) {
          console.error(`Error selecting config at path ${path}:`, error);
          return undefined as unknown as T;
        }
      }),
      distinctUntilChanged(),
      shareReplay(1)
    );
  }
  
  /**
   * Get an observable of config change events
   */
  changes$(): Observable<ConfigChangeEvent> {
    return this.changeSubject.asObservable();
  }
  
  /**
   * Get an observable of change events for a specific path
   */
  changesFor$(path: string): Observable<ConfigChangeEvent> {
    return this.changes$().pipe(
      filter(event => event.key === path || event.key.startsWith(`${path}.`)),
      shareReplay(1)
    );
  }
  
  /**
   * Get the full config object
   */
  getAll(): AppConfig {
    return this.configSubject.getValue();
  }
  
  /**
   * Get a value from the config
   */
  get<T>(key: string, defaultValue?: T): T {
    try {
      // Handle nested keys with dot notation
      const value = key.split('.').reduce((obj, k) => {
        return obj?.[k];
      }, this.configSubject.getValue() as any);
      
      return (value !== undefined ? value : defaultValue) as T;
    } catch (error) {
      console.error(`Error getting config value for ${key}:`, error);
      return defaultValue as T;
    }
  }
  
  /**
   * Set a value in the config
   */
  set<T>(key: string, value: T): void {
    try {
      // Get the current config
      const config = { ...this.configSubject.getValue() };
      const previousValue = this.get(key);
      
      // Handle nested keys with dot notation
      const keys = key.split('.');
      const lastKey = keys.pop();
      
      if (!lastKey) {
        throw new Error(`Invalid key: ${key}`);
      }
      
      // Build the nested object structure
      const obj = keys.reduce((o, k) => {
        if (o[k] === undefined) {
          o[k] = {};
        }
        return o[k];
      }, config as any);
      
      // Set the value
      obj[lastKey] = value;
      
      // Update the subject with the new config
      this.configSubject.next(config);
      
      // Emit change event
      this.changeSubject.next({
        key,
        value,
        previousValue
      });
      
      // Save to disk
      this.save();
    } catch (error) {
      console.error(`Error setting config value for ${key}:`, error);
    }
  }
  
  /**
   * Update multiple config values at once
   */
  update(updates: Record<string, any>): void {
    try {
      // Get the current config
      const config = { ...this.configSubject.getValue() };
      
      // Apply each update
      Object.entries(updates).forEach(([key, value]) => {
        const previousValue = this.get(key);
        const keys = key.split('.');
        const lastKey = keys.pop();
        
        if (!lastKey) {
          throw new Error(`Invalid key: ${key}`);
        }
        
        // Build the nested object structure
        const obj = keys.reduce((o, k) => {
          if (o[k] === undefined) {
            o[k] = {};
          }
          return o[k];
        }, config as any);
        
        // Set the value
        obj[lastKey] = value;
        
        // Emit change event
        this.changeSubject.next({
          key,
          value,
          previousValue
        });
      });
      
      // Update the subject with the new config
      this.configSubject.next(config);
      
      // Save to disk
      this.save();
    } catch (error) {
      console.error('Error updating multiple config values:', error);
    }
  }
  
  /**
   * Check if a key exists in the config
   */
  has(key: string): boolean {
    try {
      const value = key.split('.').reduce(
        (obj, k) => obj?.[k], 
        this.configSubject.getValue() as any
      );
      return value !== undefined;
    } catch (error) {
      return false;
    }
  }
  
  /**
   * Delete a key from the config
   */
  delete(key: string): void {
    try {
      // Get the current config
      const config = { ...this.configSubject.getValue() };
      
      const keys = key.split('.');
      const lastKey = keys.pop();
      
      if (!lastKey) {
        throw new Error(`Invalid key: ${key}`);
      }
      
      const obj = keys.reduce((o, k) => o?.[k], config as any);
      const previousValue = obj?.[lastKey];
      
      if (obj && obj[lastKey] !== undefined) {
        delete obj[lastKey];
        
        // Update the subject with the new config
        this.configSubject.next(config);
        
        // Emit change event
        this.changeSubject.next({
          key,
          value: undefined,
          previousValue
        });
        
        // Save to disk
        this.save();
      }
    } catch (error) {
      console.error(`Error deleting config value for ${key}:`, error);
    }
  }
  
  /**
   * Clear the config
   */
  clear(): void {
    const previousConfig = this.configSubject.getValue();
    const newConfig = { ...DEFAULT_CONFIG };
    
    // Update the subject with the default config
    this.configSubject.next(newConfig);
    
    // Emit change event for the full config
    this.changeSubject.next({
      key: '',
      value: newConfig,
      previousValue: previousConfig
    });
    
    // Save to disk
    this.save();
  }
  
  /**
   * Get the config file path
   */
  get fileName(): string {
    return this.filePath;
  }
  
  /**
   * Load data from disk
   */
  private load(): AppConfig {
    try {
      // If file exists, load and merge with defaults
      if (fs.existsSync(this.filePath)) {
        const content = fs.readFileSync(this.filePath, 'utf8');
        const data = JSON.parse(content);
        return {
          ...DEFAULT_CONFIG,
          ...data
        };
      }
      
      // Otherwise create with defaults
      fs.writeFileSync(this.filePath, JSON.stringify(DEFAULT_CONFIG, null, 2));
      return { ...DEFAULT_CONFIG };
    } catch (error) {
      console.error('Error loading config:', error);
      return { ...DEFAULT_CONFIG };
    }
  }
  
  /**
   * Save data to disk
   */
  private save(): void {
    try {
      // Ensure directory exists
      const dir = path.dirname(this.filePath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
      
      // Write file with pretty formatting
      fs.writeFileSync(
        this.filePath, 
        JSON.stringify(this.configSubject.getValue(), null, 2)
      );
    } catch (error) {
      console.error('Error saving config:', error);
    }
  }
}

// Create a singleton instance
let configInstance: ReactiveConfigStore | null = null;

/**
 * Create and initialize the configuration store
 */
export function createConfigStore() {
  if (configInstance) {
    return configInstance;
  }
  
  configInstance = new ReactiveConfigStore();
  return configInstance;
}

/**
 * Get the config store instance
 */
export function getConfigStore() {
  if (!configInstance) {
    return createConfigStore();
  }
  return configInstance;
}