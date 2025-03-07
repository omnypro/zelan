import { BaseEvent } from './BaseEvent';
import { EventCategory, SystemEventType, SystemInfoPayload } from '../../types/events';

/**
 * System info event for conveying information messages
 */
export class SystemInfoEvent extends BaseEvent<SystemInfoPayload> {
  category = EventCategory.SYSTEM;
  type = SystemEventType.INFO;

  constructor(message: string, source = 'system') {
    super({ message }, source);
  }
}

/**
 * System startup event
 */
export class SystemStartupEvent extends BaseEvent<SystemInfoPayload> {
  category = EventCategory.SYSTEM;
  type = SystemEventType.STARTUP;

  constructor(appVersion: string, source = 'system') {
    super({ appVersion }, source);
  }
}

/**
 * System error event
 */
export class SystemErrorEvent extends BaseEvent<{ message: string; error?: Error }> {
  category = EventCategory.SYSTEM;
  type = SystemEventType.ERROR;

  constructor(message: string, error?: Error, source = 'system') {
    super({ message, error }, source);
  }
}

export { BaseEvent } from './BaseEvent';
export { createEvent } from './createEvent';