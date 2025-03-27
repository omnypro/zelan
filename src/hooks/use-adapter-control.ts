import { useCallback, useEffect, useState } from 'react'
import { useTauriCommand } from './use-tauri-command'

type AdapterStatus = 'connected' | 'disconnected' | 'error' | 'disabled'

type Adapter = {
  id: string
  name: string
  description: string
  status: AdapterStatus
  statusDetails?: string
  icon?: string
}

export function useAdapterControl() {
  const [adapters, setAdapters] = useState<Adapter[]>([])
  const [loading, setLoading] = useState(true)
  
  const { invoke } = useTauriCommand()
  
  // Load adapters from backend
  useEffect(() => {
    const fetchAdapters = async () => {
      try {
        // In a real implementation, this would call a Tauri command
        // Example: const data = await invoke<Adapter[]>('get_adapter_statuses')
        
        // For the proof of concept, we'll use mock data
        setTimeout(() => {
          setAdapters([
            {
              id: 'twitch',
              name: 'Twitch',
              description: 'Connect to the Twitch API',
              status: 'connected',
              statusDetails: 'Connected as streamername'
            },
            {
              id: 'obs',
              name: 'OBS Studio',
              description: 'Connect to OBS Studio',
              status: 'disconnected'
            },
            {
              id: 'test',
              name: 'Test Adapter',
              description: 'Used for testing',
              status: 'error',
              statusDetails: 'Connection failed: invalid credentials'
            }
          ])
          setLoading(false)
        }, 1000)
      } catch (error) {
        console.error('Failed to fetch adapters:', error)
        setLoading(false)
      }
    }
    
    fetchAdapters()
  }, [invoke])
  
  // Connect adapter
  const connect = useCallback(async (adapterId: string) => {
    try {
      // In a real implementation, this would call a Tauri command
      // Example: await invoke('connect_adapter', { name: adapterId })
      
      // For the proof of concept, we'll update the state directly
      setAdapters(prev => 
        prev.map(adapter => 
          adapter.id === adapterId 
            ? { ...adapter, status: 'connected' } 
            : adapter
        )
      )
    } catch (error) {
      console.error(`Failed to connect adapter ${adapterId}:`, error)
    }
  }, [invoke])
  
  // Disconnect adapter
  const disconnect = useCallback(async (adapterId: string) => {
    try {
      // In a real implementation, this would call a Tauri command
      // Example: await invoke('disconnect_adapter', { name: adapterId })
      
      // For the proof of concept, we'll update the state directly
      setAdapters(prev => 
        prev.map(adapter => 
          adapter.id === adapterId 
            ? { ...adapter, status: 'disconnected' } 
            : adapter
        )
      )
    } catch (error) {
      console.error(`Failed to disconnect adapter ${adapterId}:`, error)
    }
  }, [invoke])
  
  return { adapters, loading, connect, disconnect }
}