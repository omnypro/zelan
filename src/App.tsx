import { useEffect } from 'react';
import { listen, type Event as TauriEvent } from '@tauri-apps/api/event';
import './App.css';

// Import components
import {
  Dashboard,
  Settings,
  ErrorNotification
} from './components';

// Import hooks
import {
  useAppState,
  useDataFetching,
  useAdapterControl
} from './hooks';

// Import types
import { ZelanError } from './types';

function App() {
  // Use app state
  const { state, dispatch } = useAppState();
  
  // Extract state variables
  const {
    eventBusStats,
    adapterStatuses,
    adapterSettings,
    testEventResult,
    wsInfo,
    activeTab,
    loading,
    errors,
    lastUpdated,
    editingAdapterConfig
  } = state;

  // Set up data fetching with error handling
  const { 
    fetchAllData, 
    sendTestEvent, 
    updateWebSocketPort 
  } = useDataFetching({
    onError: (error) => dispatch({ type: 'ADD_ERROR', payload: error }),
    onSuccess: () => dispatch({ type: 'SET_LAST_UPDATED', payload: new Date() })
  });

  // Set up adapter control with error handling
  const {
    toggleAdapterEnabled,
    updateAdapterConfig
  } = useAdapterControl({
    onError: (error) => dispatch({ type: 'ADD_ERROR', payload: error }),
    onSuccess: (message) => {
      dispatch({ 
        type: 'ADD_ERROR', 
        payload: { 
          code: 'INFO', 
          message, 
          severity: 'info' 
        } 
      });
      
      // Refresh data after adapter changes
      refreshData();
    }
  });

  // Helper function to dismiss errors
  const dismissError = (index: number) => {
    dispatch({ type: 'DISMISS_ERROR', payload: index });
  };

  // Manual refresh
  const refreshData = async () => {
    dispatch({ type: 'SET_LOADING', payload: true });
    
    try {
      const data = await fetchAllData();
      
      if (data) {
        dispatch({ type: 'SET_EVENT_BUS_STATS', payload: data.stats });
        dispatch({ type: 'SET_ADAPTER_STATUSES', payload: data.statuses });
        dispatch({ type: 'SET_ADAPTER_SETTINGS', payload: data.settings });
        dispatch({ type: 'SET_WS_INFO', payload: data.info });
        dispatch({ type: 'SET_LAST_UPDATED', payload: new Date() });
      }
    } catch (error) {
      // Errors are handled by the hook
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  // Handle sending a test event
  const handleSendTestEvent = async () => {
    dispatch({ type: 'SET_LOADING', payload: true });
    dispatch({ type: 'CLEAR_TEST_EVENT_RESULT' });
    
    try {
      const result = await sendTestEvent();
      dispatch({ type: 'SET_TEST_EVENT_RESULT', payload: result });
      await refreshData();
    } catch (error) {
      dispatch({ type: 'SET_TEST_EVENT_RESULT', payload: 'Failed to send test event' });
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  // Handle updating WebSocket port
  const handleUpdatePort = async (port: number) => {
    dispatch({ type: 'SET_LOADING', payload: true });
    
    try {
      const result = await updateWebSocketPort(port);
      
      dispatch({ 
        type: 'ADD_ERROR', 
        payload: { 
          code: 'INFO', 
          message: result, 
          severity: 'info' 
        } 
      });
      
      await refreshData();
    } catch (error) {
      // Errors are handled by the hook
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  // Handle toggling an adapter
  const handleToggleAdapter = async (adapterName: string) => {
    if (!adapterSettings) return;
    
    dispatch({ type: 'SET_LOADING', payload: true });
    
    try {
      const currentSettings = adapterSettings[adapterName];
      if (!currentSettings) {
        throw new Error(`Settings for adapter '${adapterName}' not found`);
      }
      
      await toggleAdapterEnabled(adapterName, currentSettings);
    } catch (error) {
      // Errors are handled by the hook
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  // Handle saving adapter config
  const handleSaveAdapterConfig = async (adapterName: string, configUpdates: Record<string, any>) => {
    if (!adapterSettings) return;
    
    dispatch({ type: 'SET_LOADING', payload: true });
    
    try {
      const currentSettings = adapterSettings[adapterName];
      if (!currentSettings) {
        throw new Error(`Settings for adapter '${adapterName}' not found`);
      }
      
      await updateAdapterConfig(adapterName, currentSettings, configUpdates);
      dispatch({ type: 'SET_EDITING_ADAPTER_CONFIG', payload: null });
    } catch (error) {
      // Errors are handled by the hook
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  // Fetch data on mount
  useEffect(() => {
    refreshData();
    
    // Set up auto-refresh for dashboard tab
    let intervalId: ReturnType<typeof setTimeout> | undefined;
    
    if (activeTab === 'dashboard') {
      intervalId = setInterval(() => {
        refreshData();
      }, 5000);
    }
    
    // Clean up interval on unmount or tab change
    return () => {
      if (intervalId) clearInterval(intervalId);
    };
  }, [activeTab]);

  return (
    <main className="container">
      {/* Navigation Tabs */}
      <div className="tabs">
        <button
          className={`tab-button ${activeTab === 'dashboard' ? 'active' : ''}`}
          onClick={() => dispatch({ type: 'SET_ACTIVE_TAB', payload: 'dashboard' })}
        >
          Dashboard
        </button>
        <button
          className={`tab-button ${activeTab === 'settings' ? 'active' : ''}`}
          onClick={() => dispatch({ type: 'SET_ACTIVE_TAB', payload: 'settings' })}
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
        <Dashboard
          eventBusStats={eventBusStats}
          adapterStatuses={adapterStatuses}
          wsInfo={wsInfo}
          lastUpdated={lastUpdated}
          testEventResult={testEventResult}
          loading={loading}
          onSendTestEvent={handleSendTestEvent}
          onRefreshData={refreshData}
          onUpdatePort={handleUpdatePort}
        />
      )}

      {activeTab === 'settings' && (
        <Settings
          adapterSettings={adapterSettings}
          adapterStatuses={adapterStatuses}
          loading={loading}
          editingAdapterConfig={editingAdapterConfig}
          onToggleAdapter={handleToggleAdapter}
          onConfigureAdapter={(name) => dispatch({ type: 'SET_EDITING_ADAPTER_CONFIG', payload: name })}
          onCloseConfigModal={() => dispatch({ type: 'SET_EDITING_ADAPTER_CONFIG', payload: null })}
          onSaveAdapterConfig={handleSaveAdapterConfig}
        />
      )}
    </main>
  );
}

export default App;
