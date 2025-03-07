import { useState, useEffect } from 'react';
import { useTrpc } from '@/lib/hooks/useTrpc';
import { useElectronAPI } from '@/lib/hooks/useElectronAPI';
import type { AdapterStatus } from '@/lib/trpc/shared/types';

/**
 * Demo component that shows both tRPC and direct IPC approaches side by side
 */
export function TrpcDemo() {
  // State for displaying results
  const [directResult, setDirectResult] = useState<AdapterStatus | null>(null);
  const [trpcResult, setTrpcResult] = useState<AdapterStatus | null>(null);
  const [directTiming, setDirectTiming] = useState<number>(0);
  const [trpcTiming, setTrpcTiming] = useState<number>(0);
  const [error, setError] = useState<string | null>(null);

  // Get API hooks
  const { isElectron, adapters } = useElectronAPI();
  const { isAvailable, isError, errorMessage, client } = useTrpc();

  // Fetch data on component mount
  useEffect(() => {
    // Don't do anything if not running in Electron
    if (!isElectron) return;

    // Reset error
    setError(null);

    // Fetch data using direct IPC approach
    const fetchDirect = async () => {
      try {
        const startTime = performance.now();
        const status = await adapters.getStatus('test-adapter');
        const endTime = performance.now();
        
        setDirectResult(status);
        setDirectTiming(endTime - startTime);
      } catch (err) {
        setError(`Direct IPC error: ${err instanceof Error ? err.message : String(err)}`);
      }
    };

    // Always fetch with direct IPC
    fetchDirect().catch(err => {
      console.error('Error fetching direct IPC data:', err);
    });
    
    // Only attempt tRPC if it's available and not in error state
    if (isAvailable && !isError && client) {
      const fetchTrpc = async () => {
        try {
          const startTime = performance.now();
          const status = await client.adapter.getStatus.query('test-adapter');
          const endTime = performance.now();
          
          setTrpcResult(status);
          setTrpcTiming(endTime - startTime);
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          console.error('tRPC query error:', errorMsg);
          setError(`tRPC error: ${errorMsg}`);
        }
      };
      
      fetchTrpc().catch(err => {
        console.error('Unhandled tRPC error:', err);
      });
    }

  }, [isElectron, isAvailable, isError, adapters, client]);

  // If not running in Electron, show a message
  if (!isElectron) {
    return <div className="trpc-demo">Not running in Electron environment</div>;
  }

  return (
    <div className="trpc-demo">
      <h2>API Comparison Demo</h2>
      
      {error && (
        <div className="error">
          <p>{error}</p>
        </div>
      )}
      
      {isError && errorMessage && (
        <div className="error">
          <p><strong>tRPC Initialization Error:</strong> {errorMessage}</p>
        </div>
      )}
      
      <div className="comparison">
        <div className="direct-ipc">
          <h3>Direct IPC</h3>
          <p>Time: {directTiming.toFixed(2)}ms</p>
          {directResult && (
            <div className="result">
              <p>Status: {directResult.status}</p>
              <p>Connected: {directResult.isConnected ? 'Yes' : 'No'}</p>
            </div>
          )}
        </div>
        
        <div className="trpc">
          <h3>tRPC {isAvailable ? isError ? '(Error)' : '' : '(Not Available)'}</h3>
          {isAvailable && !isError && (
            <>
              <p>Time: {trpcTiming.toFixed(2)}ms</p>
              {trpcResult && (
                <div className="result">
                  <p>Status: {trpcResult.status}</p>
                  <p>Connected: {trpcResult.isConnected ? 'Yes' : 'No'}</p>
                </div>
              )}
            </>
          )}
        </div>
      </div>
      
      <div className="actions">
        <button 
          onClick={async () => {
            if (!client || !isAvailable || isError) return;
            
            try {
              // Clear previous errors
              setError(null);
              
              // Connect adapter using tRPC
              const startTime = performance.now();
              await client.adapter.connect.mutate('test-adapter');
              const status = await client.adapter.getStatus.query('test-adapter');
              const endTime = performance.now();
              
              setTrpcResult(status);
              setTrpcTiming(endTime - startTime);
            } catch (err) {
              const errorMsg = err instanceof Error ? err.message : String(err);
              console.error('tRPC connect error:', errorMsg);
              setError(`tRPC connect error: ${errorMsg}`);
            }
          }}
          disabled={!isAvailable || isError || (trpcResult?.isConnected ?? false)}
        >
          Connect (tRPC)
        </button>
        
        <button 
          onClick={async () => {
            try {
              // Clear previous errors
              setError(null);
              
              // Connect adapter using direct IPC
              const startTime = performance.now();
              await adapters.connect('test-adapter');
              const status = await adapters.getStatus('test-adapter');
              const endTime = performance.now();
              
              setDirectResult(status);
              setDirectTiming(endTime - startTime);
            } catch (err) {
              const errorMsg = err instanceof Error ? err.message : String(err);
              console.error('Direct IPC connect error:', errorMsg);
              setError(`Direct connect error: ${errorMsg}`);
            }
          }}
          disabled={directResult?.isConnected ?? false}
        >
          Connect (Direct)
        </button>
        
        <button 
          onClick={async () => {
            if (!client || !isAvailable || isError) return;
            
            try {
              // Clear previous errors
              setError(null);
              
              // Disconnect adapter using tRPC
              const startTime = performance.now();
              await client.adapter.disconnect.mutate('test-adapter');
              const status = await client.adapter.getStatus.query('test-adapter');
              const endTime = performance.now();
              
              setTrpcResult(status);
              setTrpcTiming(endTime - startTime);
            } catch (err) {
              const errorMsg = err instanceof Error ? err.message : String(err);
              console.error('tRPC disconnect error:', errorMsg);
              setError(`tRPC disconnect error: ${errorMsg}`);
            }
          }}
          disabled={!isAvailable || isError || !(trpcResult?.isConnected ?? false)}
        >
          Disconnect (tRPC)
        </button>
        
        <button 
          onClick={async () => {
            try {
              // Clear previous errors
              setError(null);
              
              // Disconnect adapter using direct IPC
              const startTime = performance.now();
              await adapters.disconnect('test-adapter');
              const status = await adapters.getStatus('test-adapter');
              const endTime = performance.now();
              
              setDirectResult(status);
              setDirectTiming(endTime - startTime);
            } catch (err) {
              const errorMsg = err instanceof Error ? err.message : String(err);
              console.error('Direct IPC disconnect error:', errorMsg);
              setError(`Direct disconnect error: ${errorMsg}`);
            }
          }}
          disabled={!(directResult?.isConnected ?? false)}
        >
          Disconnect (Direct)
        </button>
        
        <button
          onClick={() => {
            setError(null);
            setDirectResult(null);
            setTrpcResult(null);
            setDirectTiming(0);
            setTrpcTiming(0);
            
            // Re-fetch data
            if (isElectron) {
              // Get adapter status with direct IPC
              adapters.getStatus('test-adapter')
                .then(status => {
                  setDirectResult(status);
                })
                .catch(err => {
                  console.error('Error refreshing adapter status:', err);
                });
              
              // Get adapter status with tRPC if available
              if (isAvailable && !isError && client) {
                client.adapter.getStatus.query('test-adapter')
                  .then(status => {
                    setTrpcResult(status);
                  })
                  .catch(err => {
                    console.error('Error refreshing tRPC adapter status:', err);
                  });
              }
            }
          }}
        >
          Refresh Status
        </button>
      </div>
    </div>
  );
}