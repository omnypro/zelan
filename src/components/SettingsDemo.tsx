import { useState, useEffect } from 'react';
import { useAdapterSettings } from '../lib/hooks/useAdapterSettings';
import { useUserPreferences } from '../lib/hooks/useUserPreferences';

/**
 * SettingsDemo - A component to demonstrate the persistence layer
 */
export function SettingsDemo() {
  // Get adapter settings
  const {
    settings: testAdapterSettings,
    isLoading: adapterLoading,
    error: adapterError,
    updateSettings,
    setEnabled,
    setAutoConnect
  } = useAdapterSettings('test-adapter');
  
  // Get user preferences
  const {
    ui,
    notifications,
    isLoading: preferencesLoading,
    error: preferencesError,
    setTheme,
    toggleSidebar,
    toggleNotifications
  } = useUserPreferences();
  
  // Local state for form input
  const [interval, setInterval] = useState<number>(2000);
  const [generateErrors, setGenerateErrors] = useState<boolean>(false);
  
  // Update local state when settings are loaded
  useEffect(() => {
    if (testAdapterSettings) {
      setInterval(testAdapterSettings.interval as number || 2000);
      setGenerateErrors(testAdapterSettings.generateErrors as boolean || false);
    }
  }, [testAdapterSettings]);
  
  // Save adapter settings
  const handleSaveAdapterSettings = async () => {
    if (!testAdapterSettings) return;
    
    await updateSettings({
      ...testAdapterSettings,
      interval,
      generateErrors
    });
  };
  
  return (
    <div className="settings-demo">
      <h2>Settings &amp; Persistence Demo</h2>
      
      <div className="settings-section">
        <h3>Test Adapter Settings</h3>
        {adapterError && <p className="error">Error: {adapterError}</p>}
        {adapterLoading ? (
          <p>Loading adapter settings...</p>
        ) : testAdapterSettings ? (
          <div>
            <div className="form-group">
              <label>
                <input
                  type="checkbox"
                  checked={testAdapterSettings.enabled as boolean || false}
                  onChange={(e) => setEnabled(e.target.checked)}
                />
                Enable Adapter
              </label>
            </div>
            
            <div className="form-group">
              <label>
                <input
                  type="checkbox"
                  checked={testAdapterSettings.autoConnect as boolean || false}
                  onChange={(e) => setAutoConnect(e.target.checked)}
                />
                Auto-connect on Startup
              </label>
            </div>
            
            <div className="form-group">
              <label>
                Event Interval (ms):
                <input
                  type="number"
                  min="100"
                  max="10000"
                  value={interval}
                  onChange={(e) => setInterval(parseInt(e.target.value))}
                />
              </label>
            </div>
            
            <div className="form-group">
              <label>
                <input
                  type="checkbox"
                  checked={generateErrors}
                  onChange={(e) => setGenerateErrors(e.target.checked)}
                />
                Generate Error Events
              </label>
            </div>
            
            <button onClick={handleSaveAdapterSettings}>Save Adapter Settings</button>
          </div>
        ) : (
          <p>No adapter settings available</p>
        )}
      </div>
      
      <div className="settings-section">
        <h3>User Preferences</h3>
        {preferencesError && <p className="error">Error: {preferencesError}</p>}
        {preferencesLoading ? (
          <p>Loading user preferences...</p>
        ) : (
          <div>
            <div className="form-group">
              <label>
                Theme:
                <select
                  value={ui.theme}
                  onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
                >
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                  <option value="system">System Default</option>
                </select>
              </label>
            </div>
            
            <div className="form-group">
              <label>
                <input
                  type="checkbox"
                  checked={ui.sidebarCollapsed}
                  onChange={() => toggleSidebar()}
                />
                Collapse Sidebar
              </label>
            </div>
            
            <div className="form-group">
              <label>
                <input
                  type="checkbox"
                  checked={notifications.enabled}
                  onChange={() => toggleNotifications()}
                />
                Enable Notifications
              </label>
            </div>
          </div>
        )}
      </div>
      
      <style jsx>{`
        .settings-demo {
          padding: 20px;
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
        }
        
        h2 {
          margin-top: 0;
          border-bottom: 1px solid #eee;
          padding-bottom: 10px;
        }
        
        h3 {
          margin-top: 20px;
          margin-bottom: 10px;
        }
        
        .settings-section {
          margin-bottom: 30px;
          padding: 15px;
          border: 1px solid #ddd;
          border-radius: 4px;
          background-color: #f8f8f8;
        }
        
        .form-group {
          margin-bottom: 15px;
        }
        
        label {
          display: block;
          margin-bottom: 5px;
        }
        
        input[type="checkbox"] {
          margin-right: 8px;
        }
        
        input[type="number"],
        select {
          padding: 8px;
          width: 100%;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 14px;
        }
        
        button {
          padding: 8px 16px;
          margin-top: 10px;
          border: none;
          border-radius: 4px;
          background-color: #0070f3;
          color: white;
          cursor: pointer;
          font-size: 14px;
        }
        
        button:hover {
          background-color: #0060df;
        }
        
        .error {
          color: #e53e3e;
          background-color: #fff5f5;
          padding: 10px;
          border-radius: 4px;
          border-left: 4px solid #e53e3e;
        }
      `}</style>
    </div>
  );
}