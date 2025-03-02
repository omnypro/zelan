import { ElectronAPI } from '@electron-toolkit/preload'
import { AuthState } from '@main/services/auth'

// Simple Observable interface for preload
interface SimpleObservable<T> {
  subscribe: (callback: (value: T) => void) => () => void
}

// Extended ElectronAPI with auth services
interface ExtendedElectronAPI extends ElectronAPI {
  ipcRenderer: ElectronAPI['ipcRenderer'] & {
    auth: {
      state: SimpleObservable<AuthState>
      startAuth: () => void
      logout: () => void
    }
  }
}

declare global {
  interface Window {
    electron: ExtendedElectronAPI
    api: unknown
  }
}
