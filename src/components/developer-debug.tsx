import { useState } from 'react'

export function DeveloperDebug() {
  const [activeTab, setActiveTab] = useState('events')
  const [events, setEvents] = useState<any[]>([
    { id: 1, source: 'twitch', event_type: 'stream.online', timestamp: new Date().toISOString() },
    { id: 2, source: 'obs', event_type: 'scene.changed', timestamp: new Date().toISOString() },
  ])

  const clearEvents = () => {
    setEvents([])
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">Developer Debug</h2>
        <div className="flex gap-2">
          <button
            className="px-3 py-1 bg-primary text-primary-foreground rounded hover:bg-primary/90"
            onClick={() => {
              // Send test event
              const newEvent = {
                id: events.length + 1,
                source: 'test',
                event_type: 'test.event',
                timestamp: new Date().toISOString(),
              }
              setEvents([...events, newEvent])
            }}
          >
            Send Test Event
          </button>
          <button
            className="px-3 py-1 bg-red-500 text-white rounded hover:bg-red-600"
            onClick={clearEvents}
          >
            Clear Events
          </button>
        </div>
      </div>
      
      <div className="flex border-b mb-4">
        <button
          className={`px-4 py-2 ${activeTab === 'events' ? 'border-b-2 border-primary font-medium' : 'text-gray-500'}`}
          onClick={() => setActiveTab('events')}
        >
          Event Log
        </button>
        <button
          className={`px-4 py-2 ${activeTab === 'traces' ? 'border-b-2 border-primary font-medium' : 'text-gray-500'}`}
          onClick={() => setActiveTab('traces')}
        >
          Traces
        </button>
        <button
          className={`px-4 py-2 ${activeTab === 'websocket' ? 'border-b-2 border-primary font-medium' : 'text-gray-500'}`}
          onClick={() => setActiveTab('websocket')}
        >
          WebSocket Tester
        </button>
      </div>
      
      <div className="bg-white rounded-lg shadow p-4">
        <div className="h-[60vh] overflow-auto">
          {activeTab === 'events' && (
            <div className="space-y-2">
              {events.length === 0 ? (
                <div className="text-center text-gray-500 p-4">
                  No events recorded
                </div>
              ) : (
                events.map((event) => (
                  <div key={event.id} className="p-3 border rounded hover:bg-gray-50">
                    <div className="flex justify-between">
                      <span className="font-medium">{event.event_type}</span>
                      <span className="text-sm text-gray-500">{new Date(event.timestamp).toLocaleTimeString()}</span>
                    </div>
                    <div className="text-sm">
                      Source: <span className="text-blue-600">{event.source}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}
          
          {activeTab === 'traces' && (
            <div className="text-center text-gray-500 p-4">
              Trace visualization will be implemented here
            </div>
          )}
          
          {activeTab === 'websocket' && (
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <h3 className="font-medium mb-2">WebSocket Status</h3>
                  <div className="p-3 border rounded">
                    <div className="flex items-center gap-2">
                      <div className="w-3 h-3 rounded-full bg-green-500"></div>
                      <span>Connected to: ws://localhost:9000</span>
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Send Command</h3>
                  <div className="p-3 border rounded">
                    <div className="space-y-2">
                      <select className="w-full p-2 border rounded">
                        <option value="ping">ping</option>
                        <option value="subscribe.sources">subscribe.sources</option>
                        <option value="subscribe.types">subscribe.types</option>
                        <option value="unsubscribe.all">unsubscribe.all</option>
                      </select>
                      <textarea
                        className="w-full p-2 border rounded font-mono text-sm"
                        rows={3}
                        placeholder={'{\"command\": \"subscribe.types\", \"data\": [\"stream.online\", \"stream.offline\"]}'} 
                      ></textarea>
                      <button className="w-full p-2 bg-primary text-primary-foreground rounded">
                        Send Command
                      </button>
                    </div>
                  </div>
                </div>
              </div>
              
              <div>
                <h3 className="font-medium mb-2">Messages</h3>
                <div className="border rounded p-3 font-mono text-sm bg-gray-50 h-64 overflow-y-auto">
                  <div className="text-green-600">&lt; {'{\"event_type\":\"stream.online\",\"source\":\"twitch\",\"timestamp\":\"2023-03-14T12:34:56Z\"}'}</div>
                  <div className="text-blue-600">&gt; {'{\"command\":\"ping\"}'}</div>
                  <div className="text-green-600">&lt; {'{\"response\":\"pong\"}'}</div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}