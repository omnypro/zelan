import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

function App() {
  const [eventBusStats, setEventBusStats] = useState<any>(null);
  const [adapterStatuses, setAdapterStatuses] = useState<any>(null);
  const [testEventResult, setTestEventResult] = useState<string>('');
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);

  // Fetch stats and statuses on mount and when refreshKey changes
  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);

        // Get event bus stats
        const stats = await invoke('get_event_bus_status');
        setEventBusStats(stats);

        // Get adapter statuses
        const statuses = await invoke('get_adapter_statuses');
        setAdapterStatuses(statuses);

        setLoading(false);
      } catch (error) {
        console.error('Error fetching data:', error);
        setLoading(false);
      }
    };

    fetchData();
  }, [refreshKey]);

  // Send a test event
  const sendTestEvent = async () => {
    try {
      setLoading(true);
      const result = await invoke<string>('send_test_event');
      setTestEventResult(result);
      // Refresh stats after sending an event
      setRefreshKey((prev) => prev + 1);
    } catch (error) {
      console.error('Error sending test event:', error);
      setTestEventResult(`Error: ${error}`);
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

      <div className="actions">
        <button onClick={sendTestEvent} disabled={loading}>
          {loading ? 'Processing...' : 'Send Test Event'}
        </button>
        <button onClick={refreshData} disabled={loading}>
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
          <h3>Event Bus Statistics</h3>
          {eventBusStats ? (
            <div>
              <p>
                <strong>Events Published:</strong>{' '}
                {eventBusStats.events_published}
              </p>
              <p>
                <strong>Events Dropped:</strong> {eventBusStats.events_dropped}
              </p>

              <h4>Source Counts</h4>
              <ul>
                {Object.entries(eventBusStats.source_counts || {}).map(
                  ([source, count]) => (
                    <li key={source}>
                      {source}: {count as number}
                    </li>
                  )
                )}
              </ul>

              <h4>Event Types</h4>
              <ul>
                {Object.entries(eventBusStats.type_counts || {}).map(
                  ([type, count]) => (
                    <li key={type}>
                      {type}: {count as number}
                    </li>
                  )
                )}
              </ul>
            </div>
          ) : (
            <p>Loading event bus statistics...</p>
          )}
        </div>

        <div className="stats-panel">
          <h3>Adapter Status</h3>
          {adapterStatuses ? (
            <ul>
              {Object.entries(adapterStatuses).map(([adapter, status]) => (
                <li key={adapter}>
                  <strong>{adapter}:</strong> {status as string}
                </li>
              ))}
            </ul>
          ) : (
            <p>Loading adapter statuses...</p>
          )}
        </div>
      </div>
    </main>
  );
}

export default App;
