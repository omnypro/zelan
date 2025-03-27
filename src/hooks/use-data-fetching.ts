import { useEffect, useState } from 'react'
import { useTauriCommand } from './use-tauri-command'

export function useDataFetching<T>(command: string, params?: Record<string, any>) {
  const [data, setData] = useState<T | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)
  
  const { invoke } = useTauriCommand()
  
  useEffect(() => {
    const fetchData = async () => {
      try {
        setIsLoading(true)
        
        // In a real implementation, this would call a Tauri command
        // Example: const result = await invoke<T>(command, params)
        
        // For the proof of concept, we'll simulate a delay
        setTimeout(() => {
          setIsLoading(false)
          // Returning null to simulate a successful request without data
          // In real code this would return actual data
          setData(null)
        }, 1000)
      } catch (err) {
        setError(err instanceof Error ? err : new Error(String(err)))
        setIsLoading(false)
      }
    }
    
    fetchData()
  }, [command, params, invoke])
  
  return { data, isLoading, error }
}