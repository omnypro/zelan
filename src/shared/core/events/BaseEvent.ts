import { v4 as uuidv4 } from 'uuid';
import { Event, EventCategory } from '../../types/events';

/**
 * Base event implementation that all concrete events should extend
 */
export class BaseEvent<T = unknown> implements Event<T> {
  /**
   * Unique identifier for the event
   */
  public readonly id: string;

  /**
   * Timestamp when the event was created
   */
  public readonly timestamp: number;

  /**
   * Source of the event (adapter name, system, etc.)
   */
  public readonly source: string;

  /**
   * Event category for grouping related events
   */
  public readonly category: EventCategory;

  /**
   * Specific event type within the category
   */
  public readonly type: string;

  /**
   * Event payload containing the actual data
   */
  public readonly payload: T;

  /**
   * Create a new event
   * 
   * @param source Source of the event
   * @param category Event category
   * @param type Event type
   * @param payload Event payload
   * @param id Optional custom ID (defaults to UUID)
   * @param timestamp Optional custom timestamp (defaults to Date.now())
   */
  constructor(
    source: string,
    category: EventCategory,
    type: string,
    payload: T,
    id?: string,
    timestamp?: number
  ) {
    this.id = id ?? uuidv4();
    this.timestamp = timestamp ?? Date.now();
    this.source = source;
    this.category = category;
    this.type = type;
    this.payload = payload;
  }

  /**
   * Serialize the event to a JSON string
   */
  public serialize(): string {
    return JSON.stringify(this);
  }

  /**
   * Create an event from a serialized JSON string
   * 
   * @param jsonString Serialized event
   * @returns Deserialized event
   */
  public static deserialize<P = unknown>(jsonString: string): BaseEvent<P> {
    const parsed = JSON.parse(jsonString);
    return new BaseEvent<P>(
      parsed.source,
      parsed.category,
      parsed.type,
      parsed.payload,
      parsed.id,
      parsed.timestamp
    );
  }

  /**
   * Create an event from a plain object
   * 
   * @param obj Object with event properties
   * @returns Event instance
   */
  public static fromObject<P = unknown>(obj: Event<P>): BaseEvent<P> {
    return new BaseEvent<P>(
      obj.source,
      obj.category,
      obj.type,
      obj.payload,
      obj.id,
      obj.timestamp
    );
  }
}