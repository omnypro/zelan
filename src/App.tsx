import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

// Define the ZelanError type to match what comes from the backend
interface ZelanError {
  code: string;
  message: string;
  context?: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
}

// Custom error component to display errors to the user
const ErrorNotification = ({
  error,
  onDismiss,
}: {
  error: ZelanError | string;
  onDismiss: () => void;
}) => {
  // Handle both string errors and ZelanError objects
  const errorObj =
    typeof error === 'string'
      ? {
          code: 'UNKNOWN',
          message: error,
          severity: 'error' as const,
        }
      : error;

  // Map severity to CSS class
  const severityClass =
    {
      info: 'info',
      warning: 'warning',
      error: 'error',
      critical: 'critical',
    }[errorObj.severity] || 'error';

  return (
    <div className={`error-notification ${severityClass}`}>
      <div className="error-header">
        <span className="error-code">{errorObj.code}</span>
        <button className="dismiss-button" onClick={onDismiss}>
          ×
        </button>
      </div>
      <p className="error-message">{errorObj.message}</p>
      {errorObj.context && <p className="error-context">{errorObj.context}</p>}
    </div>
  );
};

// Define WebSocketInfo interface to match backend response
interface WebSocketInfo {
  port: number;
  uri: string;
  httpUri: string;
  wscat: string;
  websocat: string;
}

// Define interfaces for adapter-related data
interface AdapterSettings {
  enabled: boolean;
  config: any;
  display_name: string;
  description: string;
}

// Maps adapter names to their settings
interface AdapterSettingsMap {
  [key: string]: AdapterSettings;
}

// Define possible service status values
type ServiceStatus = 'Connected' | 'Connecting' | 'Disconnected' | 'Error' | 'Disabled';

// Maps adapter names to their status
interface AdapterStatusMap {
  [key: string]: ServiceStatus;
}

// Tab type for navigation
type TabType = 'dashboard' | 'settings';

function App() {
  // Data states
  const [eventBusStats, setEventBusStats] = useState<any>(null);
  const [adapterStatuses, setAdapterStatuses] =
    useState<AdapterStatusMap | null>(null);
  const [adapterSettings, setAdapterSettings] =
    useState<AdapterSettingsMap | null>(null);
  const [testEventResult, setTestEventResult] = useState<string>('');
  const [wsInfo, setWsInfo] = useState<WebSocketInfo | null>(null);

  // UI states
  const [activeTab, setActiveTab] = useState<TabType>('dashboard');
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);
  const [errors, setErrors] = useState<(ZelanError | string)[]>([]);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [newPort, setNewPort] = useState<string>('');
  const [editingAdapterConfig, setEditingAdapterConfig] = useState<
    string | null
  >(null);

  // Helper function to add errors
  const addError = (error: ZelanError | string) => {
    setErrors((prev) => [error, ...prev].slice(0, 5)); // Keep only the 5 most recent errors
  };

  // Helper function to dismiss errors
  const dismissError = (index: number) => {
    setErrors((prev) => prev.filter((_, i) => i !== index));
  };

  // Helper function to handle invoke errors
  const safeInvoke = async <T,>(
    command: string,
    ...args: any[]
  ): Promise<T> => {
    try {
      return await invoke<T>(command, ...args);
    } catch (error) {
      // Handle both string errors and structured errors
      if (typeof error === 'object' && error !== null) {
        addError(error as ZelanError);
      } else {
        addError(String(error));
      }
      throw error;
    }
  };

  // Fetch stats and statuses on mount and when refreshKey changes
  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);

        // Get event bus stats
        const stats = await safeInvoke('get_event_bus_status');
        setEventBusStats(stats);

        // Get adapter statuses
        const statuses = await safeInvoke<AdapterStatusMap>(
          'get_adapter_statuses'
        );
        setAdapterStatuses(statuses);

        // Get adapter settings
        const settings = await safeInvoke<AdapterSettingsMap>(
          'get_adapter_settings'
        );
        setAdapterSettings(settings);

        // Get WebSocket info
        const info = await safeInvoke<WebSocketInfo>('get_websocket_info');
        setWsInfo(info);

        // Initialize new port state if empty
        if (newPort === '' && info?.port) {
          setNewPort(info.port.toString());
        }

        // Update last refreshed timestamp
        setLastUpdated(new Date());

        setLoading(false);
      } catch (error) {
        // Error already handled by safeInvoke
        setLoading(false);
      }
    };

    fetchData();

    // Only set up auto-refresh for dashboard tab (to avoid interfering with settings editing)
    let intervalId: ReturnType<typeof setTimeout> | undefined;

    if (activeTab === 'dashboard') {
      intervalId = setInterval(() => {
        fetchData();
      }, 5000);
    }

    // Clean up interval on unmount or tab change
    return () => {
      if (intervalId) clearInterval(intervalId);
    };
  }, [refreshKey, activeTab, newPort]);

  // Update WebSocket port
  const updatePort = async () => {
    try {
      setLoading(true);
      const port = parseInt(newPort, 10);

      if (isNaN(port) || port < 1024 || port > 65535) {
        addError('Port must be a number between 1024 and 65535');
        setLoading(false);
        return;
      }

      const result = await safeInvoke<string>('set_websocket_port', { port });

      // Show the result as a notification
      addError({
        code: 'INFO',
        message: result,
        severity: 'info',
      });

      // Refresh data to update the displayed port
      refreshData();
    } catch (error) {
      // Error already handled by safeInvoke
      setLoading(false);
    }
  };

  // Send a test event
  const sendTestEvent = async () => {
    try {
      setLoading(true);
      setTestEventResult('');

      const result = await safeInvoke<string>('send_test_event');
      setTestEventResult(result);

      // Refresh stats after sending an event
      setRefreshKey((prev) => prev + 1);
    } catch (error) {
      // Error already handled by safeInvoke
      setTestEventResult(`Failed to send test event`);
      setLoading(false);
    }
  };

  // Manual refresh
  const refreshData = () => {
    setRefreshKey((prev) => prev + 1);
  };

  // Toggle adapter enabled status
  const toggleAdapterEnabled = async (adapterName: string) => {
    if (!adapterSettings) return;

    try {
      setLoading(true);

      // Get the current settings for this adapter
      const currentSettings = adapterSettings[adapterName];
      if (!currentSettings) {
        throw new Error(`Settings for adapter '${adapterName}' not found`);
      }

      // Create updated settings with toggled enabled status
      const updatedSettings: AdapterSettings = {
        ...currentSettings,
        enabled: !currentSettings.enabled,
      };

      // Update settings on the backend
      await safeInvoke('update_adapter_settings', {
        adapterName: adapterName,
        settings: updatedSettings,
      });

      // Show success message
      addError({
        code: 'INFO',
        message: `${
          updatedSettings.enabled ? 'Enabled' : 'Disabled'
        } adapter: ${currentSettings.display_name}`,
        severity: 'info',
      });

      // Refresh data
      refreshData();
    } catch (error) {
      // Error already handled by safeInvoke
      setLoading(false);
    }
  };

  // Update adapter configuration
  const updateAdapterConfig = async (
    adapterName: string,
    configUpdates: any
  ) => {
    if (!adapterSettings) return;

    try {
      setLoading(true);

      // Get the current settings for this adapter
      const currentSettings = adapterSettings[adapterName];
      if (!currentSettings) {
        throw new Error(`Settings for adapter '${adapterName}' not found`);
      }

      // Create updated settings with merged config
      const updatedSettings: AdapterSettings = {
        ...currentSettings,
        config: {
          ...currentSettings.config,
          ...configUpdates,
        },
      };

      // Update settings on the backend
      await safeInvoke('update_adapter_settings', {
        adapterName: adapterName,
        settings: updatedSettings,
      });

      // Show success message
      addError({
        code: 'INFO',
        message: `Updated configuration for: ${currentSettings.display_name}`,
        severity: 'info',
      });

      // Refresh data
      refreshData();
    } catch (error) {
      // Error already handled by safeInvoke
      setLoading(false);
    }
  };

  return (
    <main className="container">
      {/* Navigation Tabs */}
      <div className="tabs">
        <button
          className={`tab-button ${activeTab === 'dashboard' ? 'active' : ''}`}
          onClick={() => setActiveTab('dashboard')}
        >
          Dashboard
        </button>
        <button
          className={`tab-button ${activeTab === 'settings' ? 'active' : ''}`}
          onClick={() => setActiveTab('settings')}
        >
          Settings
        </button>
      </div>

      {/* Error notifications */}
      <div className="error-container">
        {errors.map((error, index) => (
          <ErrorNotification
            key={index}
            error={error}
            onDismiss={() => dismissError(index)}
          />
        ))}
      </div>

      {/* Content based on active tab */}
      {activeTab === 'dashboard' && (
        <>
          <div className="actions">
            <button
              onClick={sendTestEvent}
              disabled={loading}
              className="action-button"
            >
              {loading ? 'Processing...' : 'Send Test Event'}
            </button>
            <button
              onClick={refreshData}
              disabled={loading}
              className="action-button"
            >
              {loading ? 'Loading...' : 'Refresh Data'}
            </button>
          </div>

          {testEventResult && (
            <div className="result-panel">
              <h3>Test Event Result</h3>
              <p>{testEventResult}</p>
            </div>
          )}
        </>
      )}

      {activeTab === 'settings' && (
        <div className="settings-section">
          <h2>Adapter Configuration</h2>
          <p className="settings-description">
            Enable, disable, and configure data adapters. Changes to adapter
            status will take effect immediately.
          </p>

          {loading ? (
            <p className="loading">Loading adapter settings...</p>
          ) : (
            <div className="adapters-grid">
              {adapterSettings &&
                Object.entries(adapterSettings).map(
                  ([adapterName, settings]) => {
                    const status =
                      adapterStatuses?.[adapterName] || 'Disconnected';
                    return (
                      <div
                        key={adapterName}
                        className={`adapter-card ${
                          settings.enabled ? '' : 'disabled'
                        }`}
                      >
                        <div className="adapter-header">
                          <h3>{settings.display_name}</h3>
                          <div
                            className={`adapter-status status-${status.toLowerCase()}`}
                          >
                            {status}
                          </div>
                        </div>

                        <p className="adapter-description">
                          {settings.description}
                        </p>

                        <div className="adapter-config">
                          {Object.entries(settings.config || {}).map(
                            ([key, value]) => (
                              <div key={key} className="config-item">
                                <span className="config-label">{key}:</span>
                                <span className="config-value">
                                  {typeof value === 'boolean'
                                    ? value
                                      ? 'Yes'
                                      : 'No'
                                    : String(value)}
                                </span>
                              </div>
                            )
                          )}
                        </div>

                        <div className="adapter-actions">
                          <button
                            className={`toggle-button ${
                              settings.enabled ? 'enabled' : 'disabled'
                            }`}
                            onClick={() => toggleAdapterEnabled(adapterName)}
                          >
                            {settings.enabled ? 'Disable' : 'Enable'}
                          </button>

                          <button
                            className="config-button"
                            onClick={() => setEditingAdapterConfig(adapterName)}
                            disabled={!settings.enabled}
                          >
                            Configure
                          </button>
                        </div>
                      </div>
                    );
                  }
                )}

              {(!adapterSettings ||
                Object.keys(adapterSettings).length === 0) && (
                <p className="no-adapters">No adapters found</p>
              )}
            </div>
          )}
        </div>
      )}

      {activeTab === 'dashboard' && (
        <div className="stats-container">
          <div className="stats-panel">
            <div className="panel-header">
              <h3>WebSocket Configuration</h3>
            </div>

            {wsInfo ? (
              <div className="websocket-info">
                <div className="websocket-connection">
                  <h4>Event Stream Connection</h4>
                  <p className="uri-display">
                    <code>{wsInfo.uri}</code>
                  </p>
                  <div className="port-configuration">
                    <div className="input-group">
                      <label htmlFor="ws-port">Port:</label>
                      <input
                        id="ws-port"
                        type="number"
                        value={newPort}
                        onChange={(e) => setNewPort(e.target.value)}
                        min="1024"
                        max="65535"
                      />
                      <button
                        onClick={updatePort}
                        disabled={loading || newPort === wsInfo.port.toString()}
                        className="action-button small"
                      >
                        Update
                      </button>
                    </div>
                    <p className="help-text">
                      Change requires app restart to take effect
                    </p>
                  </div>
                </div>

                <div className="connection-help">
                  <h4>Terminal Connection</h4>
                  <p>Connect to the event stream using:</p>
                  <pre className="terminal-command">{wsInfo.wscat}</pre>
                  <p>Or with websocat:</p>
                  <pre className="terminal-command">{wsInfo.websocat}</pre>
                  <h4>HTTP API</h4>
                  <p>REST API available at:</p>
                  <pre className="terminal-command">{wsInfo.httpUri}</pre>
                </div>
              </div>
            ) : (
              <p className="loading">Loading WebSocket configuration...</p>
            )}
          </div>

          <div className="stats-panel">
            <div className="panel-header">
              <h3>Event Bus Statistics</h3>
              {lastUpdated && (
                <span className="last-updated">
                  Last updated: {lastUpdated.toLocaleTimeString()}
                </span>
              )}
            </div>

            {eventBusStats ? (
              <div>
                <div className="stat-summary">
                  <div className="stat-box">
                    <span className="stat-value">
                      {eventBusStats.events_published}
                    </span>
                    <span className="stat-label">Events Published</span>
                  </div>
                  <div className="stat-box">
                    <span className="stat-value">
                      {eventBusStats.events_dropped}
                    </span>
                    <span className="stat-label">Events Dropped</span>
                  </div>
                </div>

                <h4>Source Counts</h4>
                <ul className="source-list">
                  {Object.entries(eventBusStats.source_counts || {}).map(
                    ([source, count]) => (
                      <li key={source} className="source-item">
                        <span className="source-name">{source}</span>
                        <span className="source-count">{count as number}</span>
                      </li>
                    )
                  )}
                  {Object.keys(eventBusStats.source_counts || {}).length ===
                    0 && <li className="empty-list">No events recorded yet</li>}
                </ul>

                <h4>Event Types</h4>
                <ul className="type-list">
                  {Object.entries(eventBusStats.type_counts || {}).map(
                    ([type, count]) => (
                      <li key={type} className="type-item">
                        <span className="type-name">{type}</span>
                        <span className="type-count">{count as number}</span>
                      </li>
                    )
                  )}
                  {Object.keys(eventBusStats.type_counts || {}).length ===
                    0 && <li className="empty-list">No events recorded yet</li>}
                </ul>
              </div>
            ) : (
              <p className="loading">Loading event bus statistics...</p>
            )}
          </div>

          <div className="stats-panel">
            <div className="panel-header">
              <h3>Adapter Status</h3>
            </div>

            {adapterStatuses ? (
              <ul className="adapter-list">
                {Object.entries(adapterStatuses).map(([adapter, status]) => {
                  const statusClass =
                    {
                      Connected: 'status-connected',
                      Connecting: 'status-connecting',
                      Disconnected: 'status-disconnected',
                      Error: 'status-error',
                    }[status as string] || '';

                  return (
                    <li key={adapter} className={`adapter-item ${statusClass}`}>
                      <span className="adapter-name">{adapter}</span>
                      <span className="adapter-status">{status as string}</span>
                    </li>
                  );
                })}
                {Object.keys(adapterStatuses).length === 0 && (
                  <li className="empty-list">No adapters registered</li>
                )}
              </ul>
            ) : (
              <p className="loading">Loading adapter statuses...</p>
            )}
          </div>
        </div>
      )}

      {/* Configuration Modal */}
      {editingAdapterConfig && adapterSettings && (
        <div className="modal-overlay">
          <div className="modal">
            <div className="modal-header">
              <h3>
                Configure {adapterSettings[editingAdapterConfig]?.display_name}
              </h3>
              <button
                className="modal-close"
                onClick={() => setEditingAdapterConfig(null)}
              >
                ×
              </button>
            </div>

            <div className="modal-content">
              <ConfigurationForm
                adapterName={editingAdapterConfig}
                config={adapterSettings[editingAdapterConfig]?.config || {}}
                onSave={(configUpdates) => {
                  updateAdapterConfig(editingAdapterConfig, configUpdates);
                  setEditingAdapterConfig(null);
                }}
                onCancel={() => setEditingAdapterConfig(null)}
              />
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

// Configuration form component for adapter settings
function ConfigurationForm({
  adapterName,
  config,
  onSave,
  onCancel,
}: {
  adapterName: string;
  config: any;
  onSave: (config: any) => void;
  onCancel: () => void;
}) {
  const [formState, setFormState] = useState<any>(config);

  // Handle input changes
  const handleInputChange = (key: string, value: any) => {
    setFormState({
      ...formState,
      [key]: value,
    });
  };

  // Convert value to appropriate type based on original value
  const convertValue = (key: string, value: string) => {
    const originalValue = config[key];

    // If original was a number, convert to number
    if (typeof originalValue === 'number') {
      return Number(value);
    }

    // If original was a boolean, convert to boolean
    if (typeof originalValue === 'boolean') {
      return value === 'true';
    }

    // Default to string
    return value;
  };

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSave(formState);
      }}
    >
      {Object.entries(config).map(([key, value]) => {
        // Render appropriate input based on value type
        if (typeof value === 'boolean') {
          return (
            <div key={key} className="form-group">
              <label>{key}:</label>
              <select
                value={formState[key].toString()}
                onChange={(e) =>
                  handleInputChange(key, e.target.value === 'true')
                }
              >
                <option value="true">Yes</option>
                <option value="false">No</option>
              </select>
            </div>
          );
        } else if (typeof value === 'number') {
          return (
            <div key={key} className="form-group">
              <label>{key}:</label>
              <input
                type="number"
                value={formState[key]}
                onChange={(e) => handleInputChange(key, Number(e.target.value))}
              />
            </div>
          );
        } else {
          return (
            <div key={key} className="form-group">
              <label>{key}:</label>
              <input
                type="text"
                value={formState[key]}
                onChange={(e) => handleInputChange(key, e.target.value)}
              />
            </div>
          );
        }
      })}

      <div className="form-actions">
        <button type="button" className="cancel-button" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="save-button">
          Save Changes
        </button>
      </div>
    </form>
  );
}

export default App;
