import { ipcMain, BrowserWindow } from 'electron';
import { observable } from '@trpc/server/observable';
import { appRouter } from '../../shared/trpc';
import { getConfigStore } from '../../shared/core/config';
import { MainEventBus } from '../services/eventBus';
import { AdapterManager } from '../services/adapters';
import { filter } from 'rxjs/operators';
import { Subject } from 'rxjs';
import { 
  createSubscriptionHandler, 
  toSerializableError, 
  createSerializableAdapter 
} from '../../shared/utils/rx-trpc';

/**
 * Context for tRPC procedures
 */
interface TRPCContext {
  mainEventBus: MainEventBus;
  adapterManager: AdapterManager;
  configStore: ReturnType<typeof getConfigStore>;
  senderIds: Set<number>;
}

/**
 * Create the tRPC context for procedure resolvers
 */
function createContext(mainEventBus: MainEventBus, adapterManager: AdapterManager): TRPCContext {
  return {
    mainEventBus,
    adapterManager,
    configStore: getConfigStore(),
    senderIds: new Set<number>()
  };
}

/**
 * Setup tRPC server over Electron IPC
 */
export function setupTRPCServer(mainEventBus: MainEventBus, adapterManager: AdapterManager) {
  const ctx = createContext(mainEventBus, adapterManager);
  
  // Channel for tRPC requests
  const TRPC_CHANNEL = 'zelan:trpc';
  
  console.log('Setting up tRPC server with channel:', TRPC_CHANNEL);
  
  // Set up IPC handler for tRPC requests
  ipcMain.handle(TRPC_CHANNEL, async (event, opts) => {
    const { id, type, path, input } = opts;
    
    console.log(`Received tRPC request: ${type} ${path}`, { id, input });
    
    try {
      // Track sender id for subscription management
      const senderId = event.sender.id;
      
      // Track the sender for cleanup
      if (!ctx.senderIds.has(senderId)) {
        ctx.senderIds.add(senderId);
        
        // Clean up when window is closed
        event.sender.once('destroyed', () => {
          ctx.senderIds.delete(senderId);
        });
      }
      
      // Parse the path
      const [moduleName, procedureName] = path.split('.');
      
      if (!moduleName || !procedureName) {
        throw new Error(`Invalid path format: ${path}. Expected "module.procedure"`);
      }
      
      console.log(`Processing ${moduleName}.${procedureName} (${type})`);
      
      // Handle the request directly based on module and procedure
      if (moduleName === 'config') {
        if (type === 'query') {
          if (procedureName === 'get') {
            const result = ctx.configStore.get(input.key, input.defaultValue);
            console.log(`Config get result for ${input.key}:`, result);
            return { id, result, type: 'data' };
          } else if (procedureName === 'getAll') {
            const result = ctx.configStore.getAll();
            console.log('Config getAll result:', result);
            return { id, result, type: 'data' };
          } else if (procedureName === 'has') {
            const result = ctx.configStore.has(input);
            return { id, result, type: 'data' };
          } else if (procedureName === 'getPath') {
            const result = ctx.configStore.fileName;
            return { id, result, type: 'data' };
          }
        } else if (type === 'mutation') {
          if (procedureName === 'set') {
            ctx.configStore.set(input.key, input.value);
            return { id, result: true, type: 'data' };
          } else if (procedureName === 'update') {
            ctx.configStore.update(input);
            return { id, result: true, type: 'data' };
          } else if (procedureName === 'delete') {
            ctx.configStore.delete(input);
            return { id, result: true, type: 'data' };
          }
        } else if (type === 'subscription') {
          const subChannelName = `${TRPC_CHANNEL}:${id}`;
          
          if (procedureName === 'onConfigChange') {
            // Create a destroyer subject for cleanup
            const destroy$ = new Subject<void>();
            
            // Use our subscription helper for cleaner code
            const subscription = createSubscriptionHandler(
              ctx.configStore.changes$(),
              // Data handler
              (data) => {
                if (!event.sender.isDestroyed()) {
                  event.sender.send(subChannelName, { id, data, type: 'data' });
                }
              },
              // Error handler
              (err) => {
                if (!event.sender.isDestroyed()) {
                  event.sender.send(subChannelName, {
                    id,
                    error: toSerializableError(err),
                    type: 'error'
                  });
                }
              }
            );
            
            // Set up cleanup
            ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
              destroy$.next();
              destroy$.complete();
              subscription.unsubscribe();
            });
            
            return { id, type: 'started' };
          } else if (procedureName === 'onPathChange') {
            const path = input;
            const subscription = ctx.configStore.changes$().subscribe({
              next: (data) => {
                if (data.key === path || data.key.startsWith(`${path}.`)) {
                  if (!event.sender.isDestroyed()) {
                    event.sender.send(subChannelName, {
                      id, data, type: 'data'
                    });
                  }
                }
              },
              error: (err) => {
                if (!event.sender.isDestroyed()) {
                  event.sender.send(subChannelName, {
                    id,
                    error: {
                      message: err.message,
                      code: 'SUBSCRIPTION_ERROR',
                      stack: err.stack,
                    },
                    type: 'error'
                  });
                }
              }
            });
            
            // Set up cleanup
            ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
              subscription.unsubscribe();
            });
            
            return { id, type: 'started' };
          }
        }
      } else if (moduleName === 'events') {
        if (type === 'mutation' && procedureName === 'send') {
          ctx.mainEventBus.publish(input);
          return { id, result: true, type: 'data' };
        } else if (type === 'subscription' && procedureName === 'onEvent') {
          const subChannelName = `${TRPC_CHANNEL}:${id}`;
          
          const subscription = ctx.mainEventBus.events$.subscribe({
            next: (data) => {
              if (!event.sender.isDestroyed()) {
                event.sender.send(subChannelName, {
                  id, data, type: 'data'
                });
              }
            },
            error: (err) => {
              if (!event.sender.isDestroyed()) {
                event.sender.send(subChannelName, {
                  id,
                  error: {
                    message: err.message,
                    code: 'SUBSCRIPTION_ERROR',
                    stack: err.stack,
                  },
                  type: 'error'
                });
              }
            }
          });
          
          // Set up cleanup
          ipcMain.once(`${TRPC_CHANNEL}:${id}:stop`, () => {
            subscription.unsubscribe();
          });
          
          return { id, type: 'started' };
        }
      } else if (moduleName === 'adapters') {
        if (type === 'query') {
          if (procedureName === 'getAll') {
            const adapters = ctx.adapterManager.getAllAdapters();
            // Use our helper to create serializable adapter objects
            const result = adapters.map(adapter => createSerializableAdapter(adapter));
            return { id, result, type: 'data' };
          } else if (procedureName === 'getById') {
            const adapter = ctx.adapterManager.getAdapter(input);
            if (!adapter) {
              return { id, result: null, type: 'data' };
            }
            // Use our helper for consistent serialization
            const result = createSerializableAdapter(adapter);
            return { id, result, type: 'data' };
          } else if (procedureName === 'getTypes') {
            const types = ctx.adapterManager.getAvailableAdapterTypes();
            // Create a plain array of strings
            const result = [...types];
            return { id, result, type: 'data' };
          }
        } else if (type === 'mutation') {
          if (procedureName === 'create') {
            const adapter = await ctx.adapterManager.createAdapter(input);
            // Use our helper for consistent serialization
            const result = createSerializableAdapter(adapter);
            return { id, result, type: 'data' };
          } else if (procedureName === 'start') {
            await ctx.adapterManager.startAdapter(input);
            return { id, result: true, type: 'data' };
          } else if (procedureName === 'stop') {
            await ctx.adapterManager.stopAdapter(input);
            return { id, result: true, type: 'data' };
          } else if (procedureName === 'update') {
            await ctx.adapterManager.updateAdapter(input.id, input.config);
            return { id, result: true, type: 'data' };
          } else if (procedureName === 'delete') {
            await ctx.adapterManager.deleteAdapter(input);
            return { id, result: true, type: 'data' };
          }
        }
      }
      
      throw new Error(`Unhandled request: ${type} ${path}`);
    } catch (error) {
      // Handle errors using our standardized error format
      console.error(`Error handling tRPC request: ${type} ${path}`, error);
      return {
        id,
        error: toSerializableError(error),
        type: 'error'
      };
    }
  });
}

/**
 * Create the tRPC router implementation with actual logic
 */
export const createTRPCRouter = (mainEventBus: MainEventBus, adapterManager: AdapterManager) => {
  const configStore = getConfigStore();
  
  return appRouter.createCaller({
    // Config router implementation
    config: {
      get: async ({ key, defaultValue }) => {
        return configStore.get(key, defaultValue);
      },
      
      set: async ({ key, value }) => {
        configStore.set(key, value);
        return true;
      },
      
      update: async (updates) => {
        configStore.update(updates);
        return true;
      },
      
      delete: async (key) => {
        configStore.delete(key);
        return true;
      },
      
      has: async (key) => {
        return configStore.has(key);
      },
      
      getAll: async () => {
        return configStore.getAll();
      },
      
      getPath: async () => {
        return configStore.fileName;
      },
      
      onConfigChange: () => {
        return observable((emit) => {
          const subscription = configStore.changes$().subscribe({
            next: (change) => emit.next(change),
            error: (err) => emit.error(err),
            complete: () => emit.complete()
          });
          
          return () => {
            subscription.unsubscribe();
          };
        });
      },
      
      onPathChange: (path) => {
        return observable((emit) => {
          // Manually filter events since we're having an issue with the rxjs operator
          const subscription = configStore.changes$().subscribe({
            next: (change) => {
              if (change.key === path || change.key.startsWith(`${path}.`)) {
                emit.next(change);
              }
            },
            error: (err) => emit.error(err),
            complete: () => emit.complete()
          });
          
          return () => {
            subscription.unsubscribe();
          };
        });
      }
    },
    
    // Events router implementation
    events: {
      send: async (event) => {
        mainEventBus.publish(event);
        return true;
      },
      
      onEvent: () => {
        return observable((emit) => {
          const subscription = mainEventBus.events$.subscribe({
            next: (event) => emit.next(event),
            error: (err) => emit.error(err),
            complete: () => emit.complete()
          });
          
          return () => {
            subscription.unsubscribe();
          };
        });
      }
    },
    
    // Adapters router implementation
    adapters: {
      getAll: async () => {
        return adapterManager.getAllAdapters();
      },
      
      getById: async (id) => {
        return adapterManager.getAdapter(id);
      },
      
      create: async (config) => {
        return adapterManager.createAdapter(config);
      },
      
      start: async (id) => {
        await adapterManager.startAdapter(id);
        return true;
      },
      
      stop: async (id) => {
        await adapterManager.stopAdapter(id);
        return true;
      },
      
      update: async ({ id, config }) => {
        await adapterManager.updateAdapter(id, config);
        return true;
      },
      
      delete: async (id) => {
        await adapterManager.deleteAdapter(id);
        return true;
      },
      
      getTypes: async () => {
        return adapterManager.getAvailableAdapterTypes();
      }
    }
  } as any);
};