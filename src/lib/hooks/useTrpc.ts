import { useEffect, useState } from 'react'
import { client } from '@/lib/trpc/client'
import { TRPCClientError } from '@trpc/client'

/**
 * Hook for using tRPC to communicate with the main process
 */
export function useTrpc() {
  // Check if we're running in Electron context with tRPC bridge
  const [isAvailable, setIsAvailable] = useState<boolean>(false)
  const [isError, setIsError] = useState<boolean>(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  // Initialize trpc client
  useEffect(() => {
    // Check if the trpcBridge is available
    const hasBridge = typeof window !== 'undefined' && 'trpcBridge' in window
    setIsAvailable(hasBridge)

    // Only proceed if the bridge is available
    if (!hasBridge) return

    // Make a test query with a delay to ensure everything is initialized
    const initTest = async () => {
      try {
        console.log('Testing tRPC client initialization...')
        // Simple test to ensure tRPC is working
        const result = await client.adapter.getStatus.query('test-adapter')
        console.log('tRPC test successful:', result)

        setIsError(false)
        setErrorMessage(null)
      } catch (err) {
        console.error('tRPC client initialization error:', err)

        // Format the error message
        let message = 'Unknown error initializing tRPC'
        if (err instanceof TRPCClientError) {
          message = err.message
        } else if (err instanceof Error) {
          message = err.message
        } else if (typeof err === 'string') {
          message = err
        }

        setIsError(true)
        setErrorMessage(message)
      }
    }

    // Add a small delay to ensure the main process has had time to initialize
    setTimeout(initTest, 500)
  }, [])

  // Return the tRPC client if available with error info
  return {
    isAvailable,
    isError,
    errorMessage,
    client: isAvailable && !isError ? client : null
  }
}
