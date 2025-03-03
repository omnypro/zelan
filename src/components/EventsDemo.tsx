import { useState } from 'react'
import { useEvents } from '../lib/hooks/useEvents'
import { useWebSocketServerTrpc } from '../lib/hooks/useWebSocketServerTrpc'
import { EventType } from '../lib/core/events/types'

/**
 * EventsDemo - A component to demonstrate the events and WebSocket integration
 */
export function EventsDemo() {
  // Hooks for events and WebSocket server
  const {
    events,
    isLoading: eventsLoading,
    error: eventsError,
    fetchRecentEvents,
    publishEvent,
    clearEvents
  } = useEvents()

  const {
    isRunning,
    clientCount,
    isLoading: wsLoading,
    error: wsError,
    startServer,
    stopServer
  } = useWebSocketServerTrpc()

  // State for custom event
  const [eventType, setEventType] = useState(EventType.SYSTEM_STARTUP)
  const [eventSource, setEventSource] = useState('demo')

  // Send a test event
  const sendTestEvent = async () => {
    try {
      const success = await publishEvent({
        id: Math.random().toString(36).substring(2, 15),
        timestamp: Date.now(),
        type: eventType,
        source: eventSource,
        data: {
          message: 'Test event from EventsDemo component',
          timestamp: new Date().toISOString()
        }
      })

      if (success) {
        // Refresh events list
        await fetchRecentEvents()
      }
    } catch (err) {
      console.error('Failed to send test event:', err)
    }
  }

  return (
    <div className="events-demo">
      <h2>Events and WebSocket Demo</h2>

      {/* WebSocket Server Controls */}
      <div className="server-controls">
        <h3>WebSocket Server</h3>
        {wsError && <p className="error">Error: {wsError}</p>}
        <p>Status: {wsLoading ? 'Loading...' : isRunning ? 'Running' : 'Stopped'}</p>
        <p>Connected Clients: {clientCount}</p>

        <div className="button-group">
          <button onClick={() => startServer()} disabled={wsLoading || isRunning}>
            Start Server
          </button>

          <button onClick={() => stopServer()} disabled={wsLoading || !isRunning}>
            Stop Server
          </button>
        </div>
      </div>

      {/* Event Publishing */}
      <div className="event-publisher">
        <h3>Publish Event</h3>

        <div className="form-group">
          <label>
            Event Type:
            <select value={eventType} onChange={(e) => setEventType(e.target.value)}>
              {Object.values(EventType).map((type) => (
                <option key={type} value={type}>
                  {type}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="form-group">
          <label>
            Event Source:
            <input
              type="text"
              value={eventSource}
              onChange={(e) => setEventSource(e.target.value)}
            />
          </label>
        </div>

        <div className="button-group">
          <button onClick={sendTestEvent}>Send Test Event</button>

          <button onClick={() => clearEvents()}>Clear Events</button>

          <button onClick={() => fetchRecentEvents()}>Refresh Events</button>
        </div>
      </div>

      {/* Events Display */}
      <div className="events-display">
        <h3>Recent Events</h3>
        {eventsError && <p className="error">Error: {eventsError}</p>}

        {eventsLoading ? (
          <p>Loading events...</p>
        ) : events.length === 0 ? (
          <p>No events yet. Try publishing one!</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Type</th>
                <th>Source</th>
                <th>Time</th>
                <th>Data</th>
              </tr>
            </thead>
            <tbody>
              {events.map((event) => (
                <tr key={event.id}>
                  <td>{event.type}</td>
                  <td>{event.source}</td>
                  <td>{new Date(event.timestamp).toLocaleTimeString()}</td>
                  <td>
                    {/* Use optional chaining and type checking for event.data */}
                    {event.data
                      ? typeof event.data === 'object'
                        ? JSON.stringify(event.data)
                        : String(event.data)
                      : 'No data'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* WebSocket Client Info */}
      <div className="websocket-client">
        <h3>WebSocket Client Connection</h3>
        <p>To connect to the WebSocket server from external clients:</p>
        <pre>
          {`
// JavaScript example
const ws = new WebSocket('ws://localhost:9090/events');

ws.onopen = () => {
  console.log('Connected to Zelan event stream');
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event received:', data);
};
          `.trim()}
        </pre>
      </div>

      <style jsx>{`
        .events-demo {
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

        .server-controls,
        .event-publisher,
        .events-display,
        .websocket-client {
          margin-bottom: 30px;
          padding: 15px;
          border: 1px solid #ddd;
          border-radius: 4px;
          background-color: #f8f8f8;
        }

        .button-group {
          margin-top: 15px;
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

        input,
        select {
          padding: 8px;
          width: 100%;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 14px;
        }

        table {
          width: 100%;
          border-collapse: collapse;
          font-size: 14px;
        }

        th,
        td {
          padding: 10px;
          text-align: left;
          border-bottom: 1px solid #ddd;
        }

        th {
          background-color: #f2f2f2;
          font-weight: 600;
        }

        tr:nth-child(even) {
          background-color: #f9f9f9;
        }

        .error {
          color: #e53e3e;
          background-color: #fff5f5;
          padding: 10px;
          border-radius: 4px;
          border-left: 4px solid #e53e3e;
        }

        pre {
          background-color: #282c34;
          color: #abb2bf;
          padding: 15px;
          border-radius: 4px;
          overflow-x: auto;
          font-size: 13px;
          line-height: 1.5;
        }
      `}</style>
    </div>
  )
}
