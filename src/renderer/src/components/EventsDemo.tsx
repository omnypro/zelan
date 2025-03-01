import React, { useEffect, useState } from 'react';
import { EventCategory, SystemEventType } from '@shared/types/events';
import { useEvents, useEventPublisher } from '@renderer/hooks/useEventStream';
import { rendererEventBus } from '@renderer/services/eventBus';
import { SystemInfoEvent } from '@shared/core/events';

/**
 * Component to demonstrate the event system
 */
export const EventsDemo: React.FC = () => {
  // Get events from the system category
  const systemEvents = useEvents<{ message?: string; appVersion?: string }>(EventCategory.SYSTEM);
  
  // Create a publisher for system info events
  const publishSystemInfo = useEventPublisher(
    EventCategory.SYSTEM, 
    SystemEventType.INFO,
    'events-demo'
  );
  
  // Input state for custom event
  const [message, setMessage] = useState('');
  
  // Publish a demo event when component mounts
  useEffect(() => {
    // Use the renderer event bus directly
    rendererEventBus.publish(
      new SystemInfoEvent('EventsDemo component mounted')
    );
    
    // Cleanup when unmounting
    return () => {
      rendererEventBus.publish(
        new SystemInfoEvent('EventsDemo component unmounted')
      );
    };
  }, []);
  
  // Handle form submission
  const handleSubmit = (e: React.FormEvent): void => {
    e.preventDefault();
    if (message.trim()) {
      // Use the publisher from the hook
      publishSystemInfo({ message });
      setMessage('');
    }
  };
  
  return (
    <div className="events-demo">
      <h2>Event System Demo</h2>
      
      <div className="event-publisher">
        <h3>Publish Event</h3>
        <form onSubmit={handleSubmit}>
          <input
            type="text"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder="Enter message for system info event"
          />
          <button type="submit">Publish Event</button>
        </form>
      </div>
      
      <div className="event-list">
        <h3>System Events ({systemEvents.length})</h3>
        <ul>
          {systemEvents.map((event) => (
            <li key={event.id}>
              <div className="event-time">
                {new Date(event.timestamp).toLocaleTimeString()}
              </div>
              <div className="event-content">
                {event.payload.message ? (
                  <span>{event.payload.message}</span>
                ) : event.payload.appVersion ? (
                  <span>System started (v{event.payload.appVersion})</span>
                ) : (
                  <span>{JSON.stringify(event.payload)}</span>
                )}
              </div>
            </li>
          ))}
        </ul>
      </div>
      
      <style>{`
        .events-demo {
          padding: 20px;
          max-width: 600px;
          margin: 0 auto;
        }
        
        .event-publisher {
          margin-bottom: 20px;
          padding: 15px;
          background: #f5f5f5;
          border-radius: 4px;
        }
        
        .event-publisher form {
          display: flex;
          gap: 10px;
        }
        
        .event-publisher input {
          flex: 1;
          padding: 8px;
          border-radius: 4px;
          border: 1px solid #ddd;
        }
        
        .event-publisher button {
          padding: 8px 16px;
          background: #0078d4;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
        }
        
        .event-list {
          background: #f9f9f9;
          border-radius: 4px;
          padding: 15px;
        }
        
        .event-list ul {
          list-style: none;
          padding: 0;
          margin: 0;
          max-height: 300px;
          overflow-y: auto;
        }
        
        .event-list li {
          padding: 8px 0;
          border-bottom: 1px solid #eee;
          display: flex;
          align-items: center;
        }
        
        .event-time {
          font-size: 0.8em;
          color: #666;
          width: 100px;
        }
        
        .event-content {
          flex: 1;
        }
      `}</style>
    </div>
  );
};

export default EventsDemo;