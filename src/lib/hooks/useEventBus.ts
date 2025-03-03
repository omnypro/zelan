import { useCallback, useEffect, useRef, useState } from 'react';
import { Observable, Subscription } from 'rxjs';
import { 
  EventBus, 
  BaseEvent, 
  EventStream 
} from '../core/events';

/**
 * Hook for interacting with the EventBus
 * Allows components to subscribe to events and publish events
 */
export function useEventBus<T extends BaseEvent = BaseEvent>(
  typeOrFilter?: string | ((event: BaseEvent) => boolean),
  source?: string
) {
  const eventBus = EventBus.getInstance();
  const [events, setEvents] = useState<T[]>([]);
  const [lastEvent, setLastEvent] = useState<T | null>(null);
  const streamRef = useRef<EventStream<T> | null>(null);
  const subscriptionRef = useRef<Subscription | null>(null);
  
  // Set up the event stream and subscription
  useEffect(() => {
    // Create event stream with the provided filter
    streamRef.current = new EventStream<T>(typeOrFilter, source);
    
    // Subscribe to the events
    subscriptionRef.current = streamRef.current.stream.subscribe(event => {
      setLastEvent(event);
      setEvents(prev => [...prev, event]);
    });
    
    // Clean up
    return () => {
      if (subscriptionRef.current) {
        subscriptionRef.current.unsubscribe();
        subscriptionRef.current = null;
      }
      
      if (streamRef.current) {
        streamRef.current.destroy();
        streamRef.current = null;
      }
    };
  }, [typeOrFilter, source]);
  
  // Publish an event to the event bus
  const publish = useCallback((event: BaseEvent) => {
    eventBus.publish(event);
  }, []);
  
  // Clear the events array
  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);
  
  // Get the event stream
  const getStream = useCallback((): Observable<T> => {
    if (!streamRef.current) {
      // Create a new stream if it doesn't exist
      streamRef.current = new EventStream<T>(typeOrFilter, source);
    }
    
    return streamRef.current.stream;
  }, [typeOrFilter, source]);
  
  return {
    events,         // Array of all events received
    lastEvent,      // Most recent event
    publish,        // Function to publish events
    clearEvents,    // Function to clear events array
    stream: getStream(),  // Observable stream of events
  };
}