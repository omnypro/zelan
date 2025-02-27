// Define the ZelanError type to match what comes from the backend
export interface ZelanError {
  code: string;
  message: string;
  context?: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
}

// Define WebSocketInfo interface to match backend response
export interface WebSocketInfo {
  port: number;
  uri: string;
  httpUri: string;
  wscat: string;
  websocat: string;
}

// Define interfaces for adapter-related data
export interface AdapterSettings {
  enabled: boolean;
  config: Record<string, any>;
  display_name: string;
  description: string;
}

// Maps adapter names to their settings
export interface AdapterSettingsMap {
  [key: string]: AdapterSettings;
}

// Define possible service status values
export type ServiceStatus = 'Connected' | 'Connecting' | 'Disconnected' | 'Error' | 'Disabled';

// Maps adapter names to their status
export interface AdapterStatusMap {
  [key: string]: ServiceStatus;
}

// EventBus stats interface
export interface EventBusStats {
  events_published: number;
  events_dropped: number;
  source_counts: Record<string, number>;
  type_counts: Record<string, number>;
}

// Tab type for navigation
export type TabType = 'dashboard' | 'settings';

// Application state interface
export interface AppState {
  eventBusStats: EventBusStats | null;
  adapterStatuses: AdapterStatusMap | null;
  adapterSettings: AdapterSettingsMap | null;
  testEventResult: string;
  wsInfo: WebSocketInfo | null;
  activeTab: TabType;
  loading: boolean;
  errors: (ZelanError | string)[];
  lastUpdated: Date | null;
  newPort: string;
  editingAdapterConfig: string | null;
}

// Action types for our reducer
export type AppAction =
  | { type: 'SET_EVENT_BUS_STATS'; payload: EventBusStats }
  | { type: 'SET_ADAPTER_STATUSES'; payload: AdapterStatusMap }
  | { type: 'SET_ADAPTER_SETTINGS'; payload: AdapterSettingsMap }
  | { type: 'SET_TEST_EVENT_RESULT'; payload: string }
  | { type: 'SET_WS_INFO'; payload: WebSocketInfo }
  | { type: 'SET_ACTIVE_TAB'; payload: TabType }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'ADD_ERROR'; payload: ZelanError | string }
  | { type: 'DISMISS_ERROR'; payload: number }
  | { type: 'SET_LAST_UPDATED'; payload: Date }
  | { type: 'SET_NEW_PORT'; payload: string }
  | { type: 'SET_EDITING_ADAPTER_CONFIG'; payload: string | null }
  | { type: 'CLEAR_TEST_EVENT_RESULT' };