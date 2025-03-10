import { ConfigStore } from '@s/core/config'
import { MainEventBus } from '@m/services/eventBus'
import { AdapterManager, ReconnectionManager } from '@m/services/adapters'
import { AuthService } from '@m/services/auth'
import { WebSocketService } from '@m/services/websocket'
import { getLoggingService } from '@m/services/logging'

/**
 * Context for tRPC procedures
 */
export interface TRPCContext {
  mainEventBus: MainEventBus
  adapterManager: AdapterManager
  reconnectionManager: ReconnectionManager
  configStore: ConfigStore
  webSocketService: WebSocketService
  authService: AuthService
  logger: ReturnType<typeof getLoggingService>['createLogger']
}

/**
 * Create the context for procedure resolvers
 */
export function createContext(
  mainEventBus: MainEventBus,
  adapterManager: AdapterManager,
  reconnectionManager: ReconnectionManager,
  configStore: ConfigStore,
  authService: AuthService
): TRPCContext {
  return {
    mainEventBus,
    adapterManager,
    reconnectionManager,
    configStore,
    webSocketService: WebSocketService.getInstance(mainEventBus, configStore),
    authService,
    logger: getLoggingService().createLogger
  }
}
