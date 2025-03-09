import React, { useState, useEffect } from 'react'
import { useConfig, useFullConfig, useConfigChanges } from '@r/hooks/useConfig'
import { ConfigChangeEvent } from '@s/core/config'

/**
 * A component to debug and test reactive configuration
 */
const ConfigDebugger: React.FC = () => {
  const config = useFullConfig()
  const [testKey, setTestKey] = useState('test-value')

  // Use reactive hooks for test key value
  const [savedValue, updateSavedValue] = useConfig<string>('debug-test-key', '(not found)')

  // Get config path the old way (doesn't need reactivity)
  const [configPath, setConfigPath] = useState<string>('')

  // Store error state
  const [error, setError] = useState<string | null>(null)

  // Get recent changes using the updated hook
  const recentChanges = useConfigChanges()

  // Save value to config
  const saveValue = async () => {
    try {
      setError(null)
      await updateSavedValue(testKey)
      console.info('Saved value to config:', testKey)
    } catch (e) {
      setError(`Error saving: ${e instanceof Error ? e.message : String(e)}`)
      console.error('Error saving config:', e)
    }
  }

  // Get config path
  const getConfigPath = async () => {
    try {
      const path = await window.api.config.getPath()
      setConfigPath(path || 'Unable to get path')
    } catch (e) {
      console.error('Error getting config path:', e)
      setConfigPath('Error getting path')
    }
  }

  // Load config path on mount
  useEffect(() => {
    getConfigPath()
  }, [])

  return (
    <div className="p-4 bg-yellow-50 border border-yellow-300 rounded-lg">
      <h3 className="text-lg font-semibold mb-2">Reactive Config Debugger</h3>

      {error && (
        <div className="mb-2 p-2 bg-red-100 border border-red-300 rounded text-red-800 text-sm">
          {error}
        </div>
      )}

      <div className="flex items-center gap-2 mb-3">
        <input
          type="text"
          value={testKey}
          onChange={(e) => setTestKey(e.target.value)}
          className="flex-1 px-3 py-2 border border-gray-300 rounded text-sm"
          placeholder="Value to save"
        />
        <button onClick={saveValue} className="px-3 py-2 bg-blue-500 text-white rounded text-sm">
          Save
        </button>
      </div>

      <div className="text-sm">
        <div className="font-medium">Current saved value (reactive):</div>
        <div className="p-2 bg-white border border-gray-300 rounded mt-1 break-all">
          {savedValue}
        </div>
      </div>

      <div className="mt-3">
        <div className="text-sm font-medium">Recent Changes:</div>
        <div className="mt-1 p-2 bg-white border border-gray-300 rounded max-h-40 overflow-auto">
          {recentChanges.length > 0 ? (
            <ul className="text-xs">
              {recentChanges.map((change, index) => (
                <li key={index} className="mb-2 pb-1 border-b border-gray-200 last:border-b-0">
                  <div>
                    <strong>Path:</strong> {change.key || '(root)'}
                  </div>
                  <div>
                    <strong>Value:</strong> {JSON.stringify(change.value)}
                  </div>
                  {change.previousValue !== undefined && (
                    <div>
                      <strong>Previous:</strong> {JSON.stringify(change.previousValue)}
                    </div>
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <div className="text-xs text-gray-500">No changes detected yet</div>
          )}
        </div>
      </div>

      <div className="mt-3 text-xs text-gray-500">Config path: {configPath}</div>

      <div className="mt-3">
        <div className="text-sm font-medium">All Configuration (reactive):</div>
        <pre className="mt-1 text-xs p-2 bg-white border border-gray-300 rounded max-h-40 overflow-auto">
          {JSON.stringify(config, null, 2) || 'No configuration data available'}
        </pre>
      </div>
    </div>
  )
}

export default ConfigDebugger
