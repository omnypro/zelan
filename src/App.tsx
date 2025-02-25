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

function App() {
  const [eventBusStats, setEventBusStats] = useState<any>(null);
  const [adapterStatuses, setAdapterStatuses] = useState<any>(null);
  const [testEventResult, setTestEventResult] = useState<string>('');
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);
  const [errors, setErrors] = useState<(ZelanError | string)[]>([]);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [wsInfo, setWsInfo] = useState<WebSocketInfo | null>(null);
  const [newPort, setNewPort] = useState<string>('');

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
        const statuses = await safeInvoke('get_adapter_statuses');
        setAdapterStatuses(statuses);

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

    // Set up auto-refresh interval (every 5 seconds)
    const intervalId = setInterval(() => {
      fetchData();
    }, 5000);

    // Clean up interval on unmount
    return () => clearInterval(intervalId);
  }, [refreshKey]);

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

  return (
    <main className="container">
      <h1>Zelan - Streaming Data Hub</h1>

      <div className="row">
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
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
                {Object.keys(eventBusStats.type_counts || {}).length === 0 && (
                  <li className="empty-list">No events recorded yet</li>
                )}
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
    </main>
  );
}

export default App;
