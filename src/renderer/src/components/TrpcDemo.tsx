import { useState, useEffect } from 'react';
import { useTrpcConfig, useTrpcEvents, useTrpcAdapters } from '../hooks/useTrpc';

/**
 * Component to demonstrate tRPC functionality
 */
export function TrpcDemo() {
  // Check if tRPC and required properties are available
  const [isTrpcAvailable] = useState<boolean>(() => {
    return !!(
      window.trpc && 
      window.trpc.config && 
      window.trpc.config.get && 
      window.trpc.events && 
      window.trpc.adapters
    );
  });
  
  // Use tRPC hooks for config
  const [theme, setTheme] = useTrpcConfig<'light' | 'dark' | 'system'>('settings.theme', 'system');
  
  // Use enhanced tRPC hooks for events with status information
  const { events, isConnected, error: eventsError, clearEvents } = useTrpcEvents();
  
  // Use tRPC hooks for adapters
  const adapters = useTrpcAdapters();
  
  // Local state
  const [newEventText, setNewEventText] = useState('');
  
  // Send an event
  const sendEvent = async () => {
    if (!window.trpc || !newEventText.trim()) return;
    
    try {
      await window.trpc.events.send.mutate({
        type: 'demo',
        payload: { message: newEventText }
      });
      setNewEventText('');
    } catch (error) {
      console.error('Error sending event:', error);
    }
  };
  
  // Log debugging info about trpc object
  useEffect(() => {
    console.log('TrpcDemo component rendered, inspecting trpc object:');
    if (window.trpc) {
      console.log('window.trpc exists:', typeof window.trpc);
      
      // Log top-level keys
      const keys = Object.keys(window.trpc || {});
      console.log('window.trpc keys:', keys);
      
      // Check config module
      if (window.trpc.config) {
        console.log('window.trpc.config exists:', typeof window.trpc.config);
        console.log('window.trpc.config keys:', Object.keys(window.trpc.config || {}));
        
        // Check config.get
        if (window.trpc.config.get) {
          console.log('window.trpc.config.get exists:', typeof window.trpc.config.get);
          console.log('window.trpc.config.get keys:', Object.keys(window.trpc.config.get || {}));
        } else {
          console.log('window.trpc.config.get is missing');
        }
      } else {
        console.log('window.trpc.config is missing');
      }
    } else {
      console.log('window.trpc is missing entirely');
    }
  }, []);
  
  // Display a message if tRPC is not available
  if (!isTrpcAvailable) {
    const missingParts = [];
    if (!window.trpc) missingParts.push('tRPC client');
    else {
      if (!window.trpc.config) missingParts.push('config module');
      else if (!window.trpc.config.get) missingParts.push('config.get');
      
      if (!window.trpc.events) missingParts.push('events module');
      if (!window.trpc.adapters) missingParts.push('adapters module');
    }

    return (
      <div className="p-4 bg-background rounded-lg shadow">
        <h2 className="text-xl font-bold mb-4">tRPC Demo</h2>
        
        <div className="bg-red-100 border-l-4 border-red-500 text-red-700 p-4 rounded">
          <p className="font-medium">tRPC client not fully available</p>
          <p className="text-sm mb-2">
            {missingParts.length > 0 
              ? `Missing parts: ${missingParts.join(', ')}`
              : 'The tRPC client is not properly initialized'}
          </p>
          <p className="text-sm">
            This could be because the tRPC server is not running or there was an error during initialization.
            Check the console for more details.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 bg-background rounded-lg shadow">
      <h2 className="text-xl font-bold mb-4">tRPC Demo</h2>
      
      <div className="space-y-6">
        {/* Config Example */}
        <div className="bg-muted p-4 rounded-md">
          <h3 className="font-medium mb-2">Config via tRPC</h3>
          <div className="flex items-center gap-4">
            <div>
              <label className="block text-sm">Theme:</label>
              <select
                value={theme}
                onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
                className="mt-1 block w-full rounded-md border-gray-300 shadow-sm bg-background"
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">System</option>
              </select>
            </div>
            <div className="text-sm">
              Current value: <code className="px-1 py-0.5 bg-background rounded">{theme}</code>
            </div>
          </div>
        </div>
        
        {/* Event Example */}
        <div className="bg-muted p-4 rounded-md">
          <h3 className="font-medium mb-2">Events via tRPC</h3>
          <div className="flex gap-2 mb-4">
            <input
              type="text"
              value={newEventText}
              onChange={(e) => setNewEventText(e.target.value)}
              className="flex-1 rounded-md border-gray-300 shadow-sm bg-background"
              placeholder="Event text..."
            />
            <button
              onClick={sendEvent}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md"
            >
              Send
            </button>
          </div>
          
          <div className="flex justify-between items-center mb-2">
            <div className="text-sm">Recent Events:</div>
            <div className="flex items-center gap-2">
              <div className={`h-2 w-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} 
                title={isConnected ? 'Connected' : 'Disconnected'} />
              <button 
                onClick={clearEvents}
                className="px-2 py-1 text-xs bg-slate-500 text-white rounded-md"
              >
                Clear
              </button>
            </div>
          </div>
          
          <div className="max-h-40 overflow-y-auto bg-background p-2 rounded-md">
            {eventsError ? (
              <div className="text-sm text-red-500 p-2">
                Error: {eventsError.message}
              </div>
            ) : !events || events.length === 0 ? (
              <div className="text-sm text-muted-foreground p-2">No events yet</div>
            ) : (
              <ul className="space-y-1">
                {events.map((event: any, index) => (
                  <li key={index} className="text-xs border-b border-muted-foreground/20 pb-1">
                    <div><span className="font-mono">{event?.type || 'Unknown Type'}</span></div>
                    <div className="text-muted-foreground">
                      {event?.payload ? JSON.stringify(event.payload) : 'No payload'}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
        
        {/* Adapter Example */}
        <div className="bg-muted p-4 rounded-md">
          <h3 className="font-medium mb-2">Adapters via tRPC</h3>
          
          <div className="mb-2 text-sm">Registered Adapters:</div>
          <div className="bg-background p-2 rounded-md">
            {adapters.length === 0 ? (
              <div className="text-sm text-muted-foreground p-2">No adapters found</div>
            ) : (
              <ul className="space-y-2">
                {adapters.map((adapter: any) => (
                  <li key={adapter.id} className="text-sm border-b border-muted-foreground/20 pb-2">
                    <div className="font-medium">{adapter.name}</div>
                    <div className="text-xs text-muted-foreground">ID: {adapter.id}</div>
                    <div className="text-xs text-muted-foreground">Type: {adapter.type}</div>
                    <div className="text-xs flex gap-1 mt-1">
                      <span className={`px-1.5 py-0.5 rounded-full text-xs ${
                        adapter.status === 'running' 
                          ? 'bg-green-200 text-green-800'
                          : adapter.status === 'stopped'
                          ? 'bg-orange-200 text-orange-800'
                          : 'bg-red-200 text-red-800'
                      }`}>
                        {adapter.status}
                      </span>
                    </div>
                    
                    <div className="flex gap-1 mt-2">
                      <button
                        onClick={() => window.trpc.adapters.start.mutate(adapter.id)}
                        disabled={adapter.status === 'running'}
                        className="px-2 py-1 text-xs bg-green-500 text-white rounded-md disabled:opacity-50"
                      >
                        Start
                      </button>
                      <button
                        onClick={() => window.trpc.adapters.stop.mutate(adapter.id)}
                        disabled={adapter.status !== 'running'}
                        className="px-2 py-1 text-xs bg-orange-500 text-white rounded-md disabled:opacity-50"
                      >
                        Stop
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}