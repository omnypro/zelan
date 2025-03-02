import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'
import type { AuthState } from '@main/services/auth'

// Custom APIs for renderer
const api = {}

// Extend Electron API with IPC for auth
const enhancedElectronAPI = {
  ...electronAPI,
  ipcRenderer: {
    ...electronAPI.ipcRenderer,
    // Add auth service API
    auth: {
      // Create an observable for auth state using the renderer event bus
      state: {
        subscribe: (callback: (state: AuthState) => void) => {
          const listener = (_: unknown, state: AuthState): void => callback(state)
          ipcRenderer.on('auth:state', listener)
          return (): void => {
            ipcRenderer.removeListener('auth:state', listener)
          }
        }
      },
      // Methods to call auth service
      startAuth: (): void => {
        ipcRenderer.send('auth:start')
      },
      logout: (): void => {
        ipcRenderer.send('auth:logout')
      }
    }
  }
}

// Use `contextBridge` APIs to expose Electron APIs to
// renderer only if context isolation is enabled, otherwise
// just add to the DOM global.
if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', enhancedElectronAPI)
    contextBridge.exposeInMainWorld('api', api)
  } catch (error) {
    console.error(error)
  }
} else {
  // @ts-ignore (define in dts)
  window.electron = enhancedElectronAPI
  // @ts-ignore (define in dts)
  window.api = api
}
