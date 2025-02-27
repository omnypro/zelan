import React from 'react';
import { EventBusStats as EventBusStatsType, AdapterStatusMap, WebSocketInfo as WebSocketInfoType } from '../types';
import EventBusStats from './EventBusStats';
import AdapterStatusList from './AdapterStatusList';
import WebSocketInfo from './WebSocketInfo';

interface DashboardProps {
  eventBusStats: EventBusStatsType | null;
  adapterStatuses: AdapterStatusMap | null;
  wsInfo: WebSocketInfoType | null;
  lastUpdated: Date | null;
  testEventResult: string;
  loading: boolean;
  onSendTestEvent: () => Promise<void>;
  onRefreshData: () => void;
  onUpdatePort: (port: number) => Promise<void>;
}

/**
 * Dashboard component displays the main monitoring interface
 */
const Dashboard: React.FC<DashboardProps> = ({
  eventBusStats,
  adapterStatuses,
  wsInfo,
  lastUpdated,
  testEventResult,
  loading,
  onSendTestEvent,
  onRefreshData,
  onUpdatePort,
}) => {
  return (
    <>
      <div className="actions">
        <button
          onClick={onSendTestEvent}
          disabled={loading}
          className="action-button"
        >
          {loading ? 'Processing...' : 'Send Test Event'}
        </button>
        <button
          onClick={onRefreshData}
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
            <WebSocketInfo 
              wsInfo={wsInfo} 
              onUpdatePort={onUpdatePort}
              loading={loading}
            />
          ) : (
            <p className="loading">Loading WebSocket configuration...</p>
          )}
        </div>

        <div className="stats-panel">
          {eventBusStats ? (
            <EventBusStats 
              stats={eventBusStats} 
              lastUpdated={lastUpdated}
            />
          ) : (
            <div className="panel-header">
              <h3>Event Bus Statistics</h3>
              <p className="loading">Loading event bus statistics...</p>
            </div>
          )}
        </div>

        <div className="stats-panel">
          {adapterStatuses ? (
            <AdapterStatusList adapterStatuses={adapterStatuses} />
          ) : (
            <div className="panel-header">
              <h3>Adapter Status</h3>
              <p className="loading">Loading adapter statuses...</p>
            </div>
          )}
        </div>
      </div>
    </>
  );
};

export default Dashboard;