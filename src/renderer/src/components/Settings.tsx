import React, { useState, useEffect } from 'react';
import { useSettings, useConfig, useTheme, useConfigChanges } from '../hooks/useConfig';
import { ConfigChangeEvent } from '../../../shared/core/config';
import ConfigDebugger from './ConfigDebugger';
import AdapterCreationForm from './AdapterCreationForm';
import AdapterList, { Adapter } from './AdapterList';
import { useTrpcAdapters, useCreateAdapter } from '../hooks/useTrpc';
import { AdapterStatus } from '../../../shared/adapters/interfaces/AdapterStatus';

/**
 * Component to display and manage application settings
 */
const Settings: React.FC = () => {
  // Use reactive hooks for settings
  const [settings, updateSettings] = useSettings();
  
  // Individual reactive settings
  const [startOnBoot] = useConfig<boolean>('settings.startOnBoot', false);
  const [minimizeToTray] = useConfig<boolean>('settings.minimizeToTray', true);
  const theme = useTheme();
  
  // Config path doesn't need reactivity
  const [configPath, setConfigPath] = useState<string>('');
  
  // Get recent settings changes
  const recentChanges = useConfigChanges('settings');
  
  // Adapter hooks
  const adapters = useTrpcAdapters();
  const { createAdapter, isCreating, error: createError } = useCreateAdapter();
  
  // Track loading state for adapter actions
  const [adapterActionLoading, setAdapterActionLoading] = useState<string | null>(null);
  
  // Transform adapters for the AdapterList component
  const formattedAdapters: Adapter[] = adapters.map(adapter => {
    return {
      id: adapter.id || '',
      type: adapter.type || '',
      name: adapter.name || '',
      enabled: adapter.enabled || false,
      status: adapter.status ? {
        status: adapter.status.status,
        message: adapter.status.message
      } : undefined,
      options: adapter.options || {}
    };
  });

  // Get the config file path
  useEffect(() => {
    const getConfigPath = async () => {
      try {
        const path = await window.api.config.getPath();
        setConfigPath(path || 'Unable to get path');
      } catch (error) {
        console.error('Failed to get config path:', error);
      }
    };
    
    getConfigPath();
  }, []);

  const handleToggleStartOnBoot = () => {
    updateSettings({
      ...settings,
      startOnBoot: !settings.startOnBoot
    });
  };

  const handleToggleMinimizeToTray = () => {
    updateSettings({
      ...settings,
      minimizeToTray: !settings.minimizeToTray
    });
  };

  const handleThemeChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    updateSettings({
      ...settings,
      theme: event.target.value as 'light' | 'dark' | 'system'
    });
  };

  return (
    <div className="p-6">
      <h2 className="text-2xl font-bold mb-6">Settings</h2>

      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <h3 className="text-lg font-semibold mb-4">Application Settings</h3>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <label className="font-medium">Start on Boot</label>
              <p className="text-sm text-gray-500">Launch Zelan when your computer starts</p>
            </div>
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={settings.startOnBoot}
                onChange={handleToggleStartOnBoot}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
            </label>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <label className="font-medium">Minimize to Tray</label>
              <p className="text-sm text-gray-500">Keep Zelan running in the system tray when closed</p>
            </div>
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={settings.minimizeToTray}
                onChange={handleToggleMinimizeToTray}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
            </label>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <label className="font-medium">Theme</label>
              <p className="text-sm text-gray-500">Choose your preferred appearance</p>
            </div>
            <select
              value={settings.theme}
              onChange={handleThemeChange}
              className="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-2.5"
            >
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="system">Follow System</option>
            </select>
          </div>
        </div>
      </div>

      {/* Reactive Config Demonstration */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
        <h3 className="text-md font-semibold mb-2">Reactive Settings Demo</h3>
        <p className="text-sm mb-4">These values update automatically when settings change:</p>
        
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-white p-3 rounded-lg shadow-sm">
            <div className="text-sm font-medium">Start on Boot</div>
            <div className="mt-1 text-lg">{startOnBoot ? 'Enabled' : 'Disabled'}</div>
          </div>
          
          <div className="bg-white p-3 rounded-lg shadow-sm">
            <div className="text-sm font-medium">Minimize to Tray</div>
            <div className="mt-1 text-lg">{minimizeToTray ? 'Enabled' : 'Disabled'}</div>
          </div>
          
          <div className="bg-white p-3 rounded-lg shadow-sm">
            <div className="text-sm font-medium">Theme</div>
            <div className="mt-1 text-lg capitalize">{theme}</div>
          </div>
        </div>
        
        <div className="mt-4">
          <h4 className="text-sm font-medium mb-2">Recent Settings Changes:</h4>
          <div className="bg-white p-3 rounded-lg shadow-sm max-h-32 overflow-auto">
            {recentChanges.length > 0 ? (
              <ul className="text-xs">
                {recentChanges.map((change, index) => (
                  <li key={index} className="mb-2 pb-1 border-b border-gray-100 last:border-b-0">
                    <div><strong>{change.key.replace('settings.', '')}</strong>: {JSON.stringify(change.value)}</div>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-sm text-gray-500">No changes detected yet</p>
            )}
          </div>
        </div>
      </div>

      {/* Configuration Debugger */}
      <div className="mb-6">
        <ConfigDebugger />
      </div>

      {/* Adapters Section */}
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <h3 className="text-lg font-semibold mb-4">Service Adapters</h3>
        <p className="text-sm text-gray-500 mb-4">
          Adapters connect to external services like OBS Studio or Twitch to receive and process events.
        </p>
        
        <div className="mb-6">
          <h4 className="text-md font-medium mb-3">Current Adapters</h4>
          <AdapterList 
            adapters={formattedAdapters} 
            loadingAdapterId={adapterActionLoading}
            onStartAdapter={async (id) => {
              setAdapterActionLoading(id);
              try {
                await window.trpc.adapters.start.mutate(id);
                // Wait longer for the status to propagate
                await new Promise(resolve => setTimeout(resolve, 1000));
                // Fetch adapters manually to ensure we have the latest state
                await window.trpc.adapters.getAll.query();
              } catch (err) {
                console.error('Failed to start adapter:', err);
                throw err;
              } finally {
                setAdapterActionLoading(null);
              }
            }}
            onStopAdapter={async (id) => {
              setAdapterActionLoading(id);
              try {
                await window.trpc.adapters.stop.mutate(id);
                // Wait longer for the status to propagate
                await new Promise(resolve => setTimeout(resolve, 1000));
                // Fetch adapters manually to ensure we have the latest state
                await window.trpc.adapters.getAll.query();
              } catch (err) {
                console.error('Failed to stop adapter:', err);
                throw err;
              } finally {
                setAdapterActionLoading(null);
              }
            }}
            onDeleteAdapter={async (id) => {
              setAdapterActionLoading(id);
              try {
                await window.trpc.adapters.delete.mutate(id);
              } catch (err) {
                console.error('Failed to delete adapter:', err);
                throw err;
              } finally {
                setAdapterActionLoading(null);
              }
            }}
          />
        </div>
        
        <div className="mt-6">
          <h4 className="text-md font-medium mb-3">Add New Adapter</h4>
          <AdapterCreationForm 
            onCreateAdapter={async (config) => {
              try {
                await createAdapter(config);
              } catch (err) {
                console.error('Failed to create adapter:', err);
                throw err;
              }
            }} 
          />
        </div>
      </div>
      
      {/* Debug information */}
      <div className="bg-gray-50 border border-gray-200 rounded-lg p-4 mt-6">
        <h3 className="text-md font-semibold mb-2">Config Location</h3>
        <p className="text-sm text-gray-600">Config Path: {configPath}</p>
      </div>
    </div>
  );
};

export default Settings;