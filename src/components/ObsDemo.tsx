import { useState, useEffect } from 'react'
import { useAdapter } from '../lib/hooks/useAdapter'
import { useEvents } from '../lib/hooks/useEvents'
import { ObsEventType } from '../lib/core/adapters/obsAdapter'

/**
 * ObsDemo - A component to demonstrate the OBS Studio adapter functionality
 */
export function ObsDemo() {
  const [currentScene, setCurrentScene] = useState<string>('Unknown')
  const [streamStatus, setStreamStatus] = useState({
    streaming: false,
    recording: false,
    duration: 0,
    kbitsPerSec: 0
  })
  
  // Get adapter status and controls
  const {
    status,
    isLoading: adapterLoading,
    error: adapterError,
    connect,
    disconnect,
    updateConfig
  } = useAdapter('obs-adapter')
  
  // Get events for this adapter
  const {
    events,
    isLoading: eventsLoading,
    error: eventsError,
    fetchRecentEvents
  } = useEvents({
    source: 'obs-adapter',
    count: 20
  })
  
  // Process events to update UI state
  useEffect(() => {
    if (!events.length) return
    
    // Find the most recent scene change event
    const sceneChangeEvent = events.find(event => event.type === ObsEventType.SCENE_CHANGED)
    if (sceneChangeEvent && 'sceneName' in sceneChangeEvent) {
      setCurrentScene(sceneChangeEvent.sceneName as string)
    }
    
    // Find the most recent streaming status event
    const streamStatusEvent = events.find(event => event.type === 'obs.streaming.status')
    if (streamStatusEvent) {
      const streamData = streamStatusEvent
      setStreamStatus({
        streaming: streamData.streaming as boolean || false,
        recording: streamData.recording as boolean || false,
        duration: streamData.duration as number || 0,
        kbitsPerSec: streamData.kbitsPerSec as number || 0
      })
    }
  }, [events])
  
  // Handle address/port config update
  const [obsAddress, setObsAddress] = useState('localhost')
  const [obsPort, setObsPort] = useState(4455)
  const [obsPassword, setObsPassword] = useState('')
  
  // Load config values when status is available
  useEffect(() => {
    if (status && status.config) {
      setObsAddress(status.config.address || 'localhost')
      setObsPort(status.config.port || 4455)
      setObsPassword(status.config.password || '')
    }
  }, [status])
  
  // Update connection settings
  const handleUpdateConfig = () => {
    updateConfig({
      address: obsAddress,
      port: obsPort,
      password: obsPassword || undefined
    })
  }
  
  // Format duration as MM:SS
  const formatDuration = (durationInMs: number) => {
    const seconds = Math.floor(durationInMs / 1000)
    const minutes = Math.floor(seconds / 60)
    const remainingSeconds = seconds % 60
    return `${minutes.toString().padStart(2, '0')}:${remainingSeconds.toString().padStart(2, '0')}`
  }
  
  return (
    <div className="obs-demo">
      <h2>OBS Studio Integration</h2>
      
      {/* Connection Status */}
      <div className="connection-status">
        <h3>Connection Status</h3>
        {adapterError && <p className="error">Error: {adapterError}</p>}
        
        {adapterLoading ? (
          <p>Loading adapter status...</p>
        ) : status ? (
          <div>
            <p>Status: <span className={`status-${status.status}`}>{status.status}</span></p>
            <p>Connected: {status.isConnected ? 'Yes' : 'No'}</p>
            
            <div className="connection-form">
              <div className="form-group">
                <label>
                  Address:
                  <input
                    type="text"
                    value={obsAddress}
                    onChange={(e) => setObsAddress(e.target.value)}
                    placeholder="localhost"
                  />
                </label>
              </div>
              
              <div className="form-group">
                <label>
                  Port:
                  <input
                    type="number"
                    value={obsPort}
                    onChange={(e) => setObsPort(parseInt(e.target.value))}
                    placeholder="4455"
                  />
                </label>
              </div>
              
              <div className="form-group">
                <label>
                  Password:
                  <input
                    type="password"
                    value={obsPassword}
                    onChange={(e) => setObsPassword(e.target.value)}
                    placeholder="Optional"
                  />
                </label>
              </div>
              
              <div className="button-group">
                <button onClick={handleUpdateConfig}>
                  Update Connection
                </button>
                
                <button
                  onClick={() => connect()}
                  disabled={status.isConnected}
                >
                  Connect
                </button>
                
                <button
                  onClick={() => disconnect()}
                  disabled={!status.isConnected}
                >
                  Disconnect
                </button>
              </div>
            </div>
          </div>
        ) : (
          <p>No adapter status available</p>
        )}
      </div>
      
      {/* Current State */}
      <div className="obs-state">
        <h3>Current OBS State</h3>
        
        <div className="state-info">
          <div className="state-item">
            <strong>Current Scene:</strong>
            <span>{currentScene}</span>
          </div>
          
          <div className="state-item">
            <strong>Streaming:</strong>
            <span className={streamStatus.streaming ? 'active' : 'inactive'}>
              {streamStatus.streaming ? 'Live' : 'Offline'}
            </span>
          </div>
          
          <div className="state-item">
            <strong>Recording:</strong>
            <span className={streamStatus.recording ? 'active' : 'inactive'}>
              {streamStatus.recording ? 'Recording' : 'Stopped'}
            </span>
          </div>
          
          {streamStatus.streaming && (
            <>
              <div className="state-item">
                <strong>Duration:</strong>
                <span>{formatDuration(streamStatus.duration)}</span>
              </div>
              
              <div className="state-item">
                <strong>Bitrate:</strong>
                <span>{streamStatus.kbitsPerSec} kbps</span>
              </div>
            </>
          )}
        </div>
      </div>
      
      {/* Recent Events */}
      <div className="recent-events">
        <h3>Recent OBS Events</h3>
        {eventsError && <p className="error">Error: {eventsError}</p>}
        
        <div className="button-group">
          <button onClick={() => fetchRecentEvents()}>
            Refresh Events
          </button>
        </div>
        
        {eventsLoading ? (
          <p>Loading events...</p>
        ) : events.length === 0 ? (
          <p>No events yet from OBS</p>
        ) : (
          <div className="events-list">
            {events.slice(0, 10).map((event) => (
              <div key={event.id} className="event-item">
                <div className="event-type">{event.type}</div>
                <div className="event-time">{new Date(event.timestamp).toLocaleTimeString()}</div>
                <div className="event-data">
                  {event.type === ObsEventType.SCENE_CHANGED && 'sceneName' in event ? (
                    <>Scene: {event.sceneName}</>
                  ) : event.type === 'obs.streaming.status' ? (
                    <>Streaming: {event.streaming ? 'Yes' : 'No'}, Recording: {event.recording ? 'Yes' : 'No'}</>
                  ) : event.type === ObsEventType.SOURCE_ACTIVATED || event.type === ObsEventType.SOURCE_DEACTIVATED ? (
                    <>Source: {event.sourceName} - {event.visible ? 'Visible' : 'Hidden'}</>
                  ) : (
                    <>Event received</>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      
      {/* Connection Instructions */}
      <div className="connection-help">
        <h3>Connection Requirements</h3>
        <p>To use this integration, OBS Studio must have the WebSocket server enabled:</p>
        <ol>
          <li>Open OBS Studio</li>
          <li>Go to <strong>Tools &gt; WebSocket Server Settings</strong></li>
          <li>Check <strong>Enable WebSocket server</strong></li>
          <li>Default port is <strong>4455</strong></li>
          <li>Enable authentication if needed (password required)</li>
          <li>Click <strong>OK</strong> to save</li>
        </ol>
      </div>
      
      <style jsx>{`
        .obs-demo {
          padding: 20px;
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu,
            Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
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
        
        .connection-status,
        .obs-state,
        .recent-events,
        .connection-help {
          margin-bottom: 30px;
          padding: 15px;
          border: 1px solid #ddd;
          border-radius: 4px;
          background-color: #f8f8f8;
        }
        
        .button-group {
          margin: 15px 0;
        }
        
        button {
          padding: 8px 16px;
          margin-right: 10px;
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
        
        button:disabled {
          background-color: #cccccc;
          cursor: not-allowed;
        }
        
        .form-group {
          margin-bottom: 15px;
        }
        
        label {
          display: block;
          margin-bottom: 5px;
        }
        
        input {
          padding: 8px;
          width: 100%;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 14px;
        }
        
        .error {
          color: #e53e3e;
          background-color: #fff5f5;
          padding: 10px;
          border-radius: 4px;
          border-left: 4px solid #e53e3e;
        }
        
        .status-connected {
          color: #38a169;
          font-weight: bold;
        }
        
        .status-disconnected {
          color: #718096;
        }
        
        .status-connecting {
          color: #d69e2e;
        }
        
        .status-error {
          color: #e53e3e;
        }
        
        .state-info {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
          gap: 10px;
        }
        
        .state-item {
          padding: 10px;
          background-color: #fff;
          border-radius: 4px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }
        
        .state-item strong {
          display: block;
          margin-bottom: 5px;
          color: #4a5568;
        }
        
        .active {
          color: #38a169;
          font-weight: bold;
        }
        
        .inactive {
          color: #718096;
        }
        
        .events-list {
          max-height: 300px;
          overflow-y: auto;
          background-color: #fff;
          border-radius: 4px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }
        
        .event-item {
          padding: 10px;
          border-bottom: 1px solid #eee;
          display: grid;
          grid-template-columns: 1fr auto;
          grid-template-rows: auto auto;
          gap: 5px;
        }
        
        .event-item:last-child {
          border-bottom: none;
        }
        
        .event-type {
          font-weight: bold;
          color: #4a5568;
        }
        
        .event-time {
          text-align: right;
          color: #718096;
          font-size: 0.9em;
        }
        
        .event-data {
          grid-column: 1 / -1;
          color: #2d3748;
          font-size: 0.9em;
        }
        
        .connection-help ol {
          padding-left: 20px;
        }
        
        .connection-help li {
          margin-bottom: 8px;
        }
      `}</style>
    </div>
  )
}