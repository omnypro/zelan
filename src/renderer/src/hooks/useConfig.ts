import { AppConfig, ConfigChangeEvent } from '@s/core/config'
import { useState, useEffect } from 'react'

/**
 * Hook to get and set configuration values using RxJS
 */
export function useConfig<T>(key: string, defaultValue: T): [T, (value: T) => Promise<void>] {
  const [value, setValue] = useState<T>(defaultValue)

  // Subscribe to config changes for this key
  useEffect(() => {
    const unsubscribe = window.api.config.select$(key, defaultValue, (newValue: T) => {
      setValue(newValue)
    })

    return () => {
      unsubscribe()
    }
  }, [key, defaultValue])

  // Function to update the value
  const updateValue = async (newValue: T) => {
    try {
      await window.api.config.set(key, newValue)
    } catch (error) {
      console.error(`Error updating config for ${key}:`, error)
    }
  }

  return [value, updateValue]
}

/**
 * Hook to get adapter configurations
 */
export function useAdapterConfigs() {
  const [configs, setConfigs] = useState<Record<string, any>>({})

  useEffect(() => {
    const unsubscribe = window.api.config.select$('adapters', {}, (adapters) => {
      setConfigs(adapters)
    })

    return () => {
      unsubscribe()
    }
  }, [])

  return configs
}

/**
 * Hook to get application settings
 */
export function useSettings() {
  const defaultSettings: AppConfig['settings'] = {
    startOnBoot: false,
    minimizeToTray: true,
    theme: 'system'
  }

  return useConfig<AppConfig['settings']>('settings', defaultSettings)
}

/**
 * Hook to get theme setting
 */
export function useTheme() {
  const [theme, setTheme] = useState<'light' | 'dark' | 'system'>('system')

  useEffect(() => {
    const unsubscribe = window.api.config.select$('settings.theme', 'system', (newTheme) => {
      setTheme(newTheme)
    })

    return () => {
      unsubscribe()
    }
  }, [])

  return theme
}

/**
 * Hook to subscribe to configuration changes
 */
export function useConfigChanges(key?: string) {
  const [changes, setChanges] = useState<ConfigChangeEvent[]>([])

  useEffect(() => {
    const callback = (event: ConfigChangeEvent) => {
      setChanges((prev) => [event, ...prev].slice(0, 10))
    }

    const unsubscribe = key ? window.api.config.changesFor$(key, callback) : window.api.config.changes$(callback)

    return () => {
      unsubscribe()
    }
  }, [key])

  return changes
}

/**
 * Hook to get the entire config
 */
export function useFullConfig() {
  const [config, setConfig] = useState<AppConfig>({} as AppConfig)

  useEffect(() => {
    const unsubscribe = window.api.config.config$((newConfig) => {
      setConfig(newConfig)
    })

    return () => {
      unsubscribe()
    }
  }, [])

  return config
}
