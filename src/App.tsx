import { useEffect, useState } from 'react'
import reactLogo from './assets/react.svg'
import viteLogo from '/electron-vite.animate.svg'
import './App.css'
import { useElectronAPI } from './lib/hooks/useElectronAPI'
import type { AdapterStatus } from './lib/trpc/shared/types'
import { TrpcDemo } from './components/TrpcDemo'
import { EventsDemo } from './components/EventsDemo'
import { SettingsDemo } from './components/SettingsDemo'
import { ObsDemo } from './components/ObsDemo'
import './components/TrpcDemo.css'

function App() {
  const [message, setMessage] = useState<string>('Loading...')
  const [adapterStatus, setAdapterStatus] = useState<AdapterStatus | null>(null)
  const [showTrpcDemo, setShowTrpcDemo] = useState<boolean>(false)
  const [showEventsDemo, setShowEventsDemo] = useState<boolean>(false)
  const [showSettingsDemo, setShowSettingsDemo] = useState<boolean>(false)
  const [showObsDemo, setShowObsDemo] = useState<boolean>(false)

  // Use the Electron API hook instead of direct window access
  const { isElectron, adapters } = useElectronAPI()

  // Effect for setting up message listeners
  useEffect(() => {
    if (!isElectron) {
      setMessage('Not running in Electron environment')
      return
    }

    setMessage('Running in Electron!')

    // Define message handler
    const messageHandler = (messageText: string) => {
      setMessage(`Message from main process: ${messageText}`)
    }

    // Add listener
    window.ipcRenderer.on('main-process-message', messageHandler)

    // Cleanup function
    return () => {
      window.ipcRenderer.off('main-process-message', messageHandler)
    }
  }, [isElectron])

  // Separate effect for fetching adapter status
  useEffect(() => {
    if (!isElectron) return

    // Get adapter status
    adapters
      .getStatus('test-adapter')
      .then((status) => {
        setAdapterStatus(status)
      })
      .catch((err) => {
        console.error('Error getting adapter status:', err)
      })
  }, [isElectron, adapters])

  // Handler for connecting the adapter
  const handleConnectAdapter = async () => {
    try {
      await adapters.connect('test-adapter')
      const status = await adapters.getStatus('test-adapter')
      setAdapterStatus(status)
    } catch (error) {
      console.error('Error connecting adapter:', error)
    }
  }

  // Handler for disconnecting the adapter
  const handleDisconnectAdapter = async () => {
    try {
      await adapters.disconnect('test-adapter')
      const status = await adapters.getStatus('test-adapter')
      setAdapterStatus(status)
    } catch (error) {
      console.error('Error disconnecting adapter:', error)
    }
  }

  // Toggle tRPC demo visibility
  const toggleTrpcDemo = () => {
    setShowTrpcDemo((prev) => !prev)
  }

  // Toggle Events demo visibility
  const toggleEventsDemo = () => {
    setShowEventsDemo((prev) => !prev)
  }
  
  // Toggle Settings demo visibility
  const toggleSettingsDemo = () => {
    setShowSettingsDemo((prev) => !prev)
  }
  
  // Toggle OBS demo visibility
  const toggleObsDemo = () => {
    setShowObsDemo((prev) => !prev)
  }

  return (
    <>
      <div>
        <a href="https://electron-vite.github.io" target="_blank">
          <img src={viteLogo} className="logo" alt="Vite logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <h1>Zelan Data Aggregation Service</h1>

      <div className="card">
        <h2>Basic Electron Test</h2>
        <p>{message}</p>

        {isElectron && (
          <>
            <h3>Test Adapter</h3>
            {adapterStatus ? (
              <div>
                <p>Status: {adapterStatus.status}</p>
                <p>Connected: {adapterStatus.isConnected ? 'Yes' : 'No'}</p>
                <button onClick={handleConnectAdapter} disabled={adapterStatus.isConnected}>
                  Connect
                </button>
                <button onClick={handleDisconnectAdapter} disabled={!adapterStatus.isConnected}>
                  Disconnect
                </button>
              </div>
            ) : (
              <p>Loading adapter status...</p>
            )}

            <div className="demo-toggles" style={{ marginTop: '20px', textAlign: 'center' }}>
              <button onClick={toggleTrpcDemo} style={{ marginRight: '10px' }}>
                {showTrpcDemo ? 'Hide tRPC Demo' : 'Show tRPC Demo'}
              </button>

              <button onClick={toggleEventsDemo} style={{ marginRight: '10px' }}>
                {showEventsDemo ? 'Hide Events Demo' : 'Show Events Demo'}
              </button>
              
              <button onClick={toggleSettingsDemo}>
                {showSettingsDemo ? 'Hide Settings Demo' : 'Show Settings Demo'}
              </button>
              
              <button onClick={toggleObsDemo}>
                {showObsDemo ? 'Hide OBS Demo' : 'Show OBS Demo'}
              </button>
            </div>

            {showTrpcDemo && <TrpcDemo />}
            {showEventsDemo && <EventsDemo />}
            {showSettingsDemo && <SettingsDemo />}
            {showObsDemo && <ObsDemo />}
          </>
        )}
      </div>
    </>
  )
}

export default App
