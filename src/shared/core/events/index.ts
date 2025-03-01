export * from './BaseEvent'

import { BaseEvent } from './BaseEvent'
import {
  EventCategory,
  SystemEventType,
  AuthEventType,
  AdapterEventType,
  WebSocketEventType,
  SystemStartupPayload,
  SystemErrorPayload,
  AuthPayload,
  AuthSuccessPayload,
  AuthErrorPayload
} from '@shared/types/events'

/**
 * System Events
 */
export class SystemStartupEvent extends BaseEvent<SystemStartupPayload> {
  constructor(payload: SystemStartupPayload) {
    super('system', EventCategory.SYSTEM, SystemEventType.STARTUP, payload)
  }
}

export class SystemShutdownEvent extends BaseEvent<void> {
  constructor() {
    super('system', EventCategory.SYSTEM, SystemEventType.SHUTDOWN, undefined)
  }
}

export class SystemErrorEvent extends BaseEvent<SystemErrorPayload> {
  constructor(payload: SystemErrorPayload) {
    super('system', EventCategory.SYSTEM, SystemEventType.ERROR, payload)
  }
}

export class SystemInfoEvent extends BaseEvent<{ message: string }> {
  constructor(message: string) {
    super('system', EventCategory.SYSTEM, SystemEventType.INFO, { message })
  }
}

/**
 * Authentication Events
 */
export class AuthLoginStartedEvent extends BaseEvent<AuthPayload> {
  constructor(payload: AuthPayload) {
    super('auth', EventCategory.AUTH, AuthEventType.LOGIN_STARTED, payload)
  }
}

export class AuthLoginSuccessEvent extends BaseEvent<AuthSuccessPayload> {
  constructor(payload: AuthSuccessPayload) {
    super('auth', EventCategory.AUTH, AuthEventType.LOGIN_SUCCESS, payload)
  }
}

export class AuthLoginFailedEvent extends BaseEvent<AuthErrorPayload> {
  constructor(payload: AuthErrorPayload) {
    super('auth', EventCategory.AUTH, AuthEventType.LOGIN_FAILED, payload)
  }
}

export class AuthLogoutEvent extends BaseEvent<AuthPayload> {
  constructor(payload: AuthPayload) {
    super('auth', EventCategory.AUTH, AuthEventType.LOGOUT, payload)
  }
}

export class AuthTokenRefreshEvent extends BaseEvent<AuthSuccessPayload> {
  constructor(payload: AuthSuccessPayload) {
    super('auth', EventCategory.AUTH, AuthEventType.TOKEN_REFRESH, payload)
  }
}

export class AuthTokenExpiredEvent extends BaseEvent<AuthPayload> {
  constructor(payload: AuthPayload) {
    super('auth', EventCategory.AUTH, AuthEventType.TOKEN_EXPIRED, payload)
  }
}

/**
 * Adapter Events
 */
export class AdapterConnectedEvent extends BaseEvent<{ adapterId: string }> {
  constructor(adapterId: string) {
    super('adapter', EventCategory.ADAPTER, AdapterEventType.CONNECTED, { adapterId })
  }
}

export class AdapterDisconnectedEvent extends BaseEvent<{ adapterId: string }> {
  constructor(adapterId: string) {
    super('adapter', EventCategory.ADAPTER, AdapterEventType.DISCONNECTED, { adapterId })
  }
}

export class AdapterReconnectingEvent extends BaseEvent<{ adapterId: string }> {
  constructor(adapterId: string) {
    super('adapter', EventCategory.ADAPTER, AdapterEventType.RECONNECTING, { adapterId })
  }
}

export class AdapterErrorEvent extends BaseEvent<{ adapterId: string; error: string }> {
  constructor(adapterId: string, error: string) {
    super('adapter', EventCategory.ADAPTER, AdapterEventType.ERROR, { adapterId, error })
  }
}

/**
 * WebSocket Events
 */
export class WebSocketClientConnectedEvent extends BaseEvent<{ clientId: string }> {
  constructor(clientId: string) {
    super('websocket', EventCategory.WEBSOCKET, WebSocketEventType.CLIENT_CONNECTED, { clientId })
  }
}

export class WebSocketClientDisconnectedEvent extends BaseEvent<{ clientId: string }> {
  constructor(clientId: string) {
    super('websocket', EventCategory.WEBSOCKET, WebSocketEventType.CLIENT_DISCONNECTED, {
      clientId
    })
  }
}

export class WebSocketMessageReceivedEvent extends BaseEvent<{
  clientId: string
  message: unknown
}> {
  constructor(clientId: string, message: unknown) {
    super('websocket', EventCategory.WEBSOCKET, WebSocketEventType.MESSAGE_RECEIVED, {
      clientId,
      message
    })
  }
}

export class WebSocketMessageSentEvent extends BaseEvent<{ clientId: string; message: unknown }> {
  constructor(clientId: string, message: unknown) {
    super('websocket', EventCategory.WEBSOCKET, WebSocketEventType.MESSAGE_SENT, {
      clientId,
      message
    })
  }
}
